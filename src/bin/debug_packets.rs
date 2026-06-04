use albion_network_lib::{
    CaptureFilter, DecodedPacket, HostFilter, ItemNameResolver, PhotonParser, PhotonParserConfig,
    WorldMap, extract_udp_payload,
};
use pcap::{Capture, Device, Error as PcapError};
use std::{
    env,
    error::Error,
    fs,
    io::{self, Write},
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

const DEFAULT_PORTS: [u16; 2] = [5056, 4535];
const ETHERNET_LINK_TYPE: i32 = 1;

#[derive(Debug, PartialEq, Eq)]
struct CliOptions {
    interface: Option<String>,
    ports: Vec<u16>,
    any_port: bool,
    host_cidrs: Vec<String>,
    hosts_files: Vec<PathBuf>,
    debug: bool,
    unknown: bool,
    count: Option<usize>,
    no_events: bool,
    no_operations: bool,
    op_include: Option<Vec<i32>>,
    event_include: Option<Vec<i32>>,
    op_exclude: Option<Vec<i32>>,
    event_exclude: Option<Vec<i32>>,
}

impl Default for CliOptions {
    fn default() -> Self {
        Self {
            interface: None,
            ports: DEFAULT_PORTS.to_vec(),
            any_port: false,
            host_cidrs: Vec::new(),
            hosts_files: Vec::new(),
            debug: false,
            unknown: false,
            count: None,
            no_events: false,
            no_operations: false,
            op_include: None,
            event_include: None,
            op_exclude: None,
            event_exclude: None,
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args
        .iter()
        .any(|arg| matches!(arg.as_str(), "-h" | "--help"))
    {
        print_usage();
        return Ok(());
    }

    let options = parse_args(args)?;
    run(options)
}

fn run(options: CliOptions) -> Result<(), Box<dyn Error>> {
    let running = Arc::new(AtomicBool::new(true));
    let signal_running = running.clone();
    ctrlc::set_handler(move || {
        signal_running.store(false, Ordering::SeqCst);
    })?;

    let device = select_device(options.interface.as_deref())?;
    eprintln!("capturing on interface {}", device.name);

    let mut capture = Capture::from_device(device)?
        .promisc(true)
        .snaplen(4096)
        .timeout(250)
        .open()?;

    let link_type = capture.get_datalink().0;
    if link_type != ETHERNET_LINK_TYPE {
        return Err(format!(
            "unsupported datalink type {link_type}; debug_packets currently supports Ethernet only"
        )
        .into());
    }

    if let Some(filter_expression) = build_bpf_filter(&options) {
        capture.filter(&filter_expression, true)?;
        eprintln!("installed BPF filter: {filter_expression}");
    } else {
        capture.filter("udp", true)?;
        eprintln!("installed BPF filter: udp");
    }

    let capture_filter = build_capture_filter(&options)?;
    let mut parser = build_parser(&options);
    let mut stdout = io::stdout().lock();
    let mut packet_number = 0usize;
    let mut processed_packets = 0usize;
    let mut emitted_packets = 0usize;

    while running.load(Ordering::SeqCst) {
        let packet = match capture.next_packet() {
            Ok(packet) => packet,
            Err(PcapError::TimeoutExpired) => continue,
            Err(PcapError::NoMorePackets) => break,
            Err(error) => return Err(error.into()),
        };

        packet_number += 1;
        let Some(udp_packet) = extract_udp_payload(packet.data, Some(link_type as u16)) else {
            continue;
        };

        if !capture_filter.matches_udp_packet(&udp_packet) {
            continue;
        }

        processed_packets += 1;
        let decoded_before = parser.decoded_packets().len();
        if let Err(error) = parser.receive_packet(
            udp_packet.payload,
            packet_number,
            udp_packet.source,
            udp_packet.destination,
        ) {
            eprintln!("packet {packet_number}: decode error: {}", error.0);
        }

        for decoded_packet in parser.decoded_packets()[decoded_before..]
            .iter()
            .filter(|decoded_packet| should_emit(decoded_packet, &options))
        {
            serde_json::to_writer(&mut stdout, decoded_packet)?;
            stdout.write_all(b"\n")?;
            emitted_packets += 1;
        }
        stdout.flush()?;

        if options
            .count
            .is_some_and(|packet_limit| processed_packets >= packet_limit)
        {
            break;
        }
    }

    eprintln!(
        "processed {processed_packets} UDP packets; emitted {emitted_packets} decoded packets"
    );
    Ok(())
}

fn should_emit(packet: &DecodedPacket, options: &CliOptions) -> bool {
    match packet {
        DecodedPacket::Operation(operation) => {
            if options.no_operations {
                return false;
            }

            code_filter_matches(
                operation.code as i32,
                &options.op_include,
                &options.op_exclude,
            )
        }
        DecodedPacket::Event(event) => {
            if options.no_events {
                return false;
            }

            code_filter_matches(
                event.code as i32,
                &options.event_include,
                &options.event_exclude,
            )
        }
        DecodedPacket::Unknown(_) => true,
    }
}

fn code_filter_matches(code: i32, include: &Option<Vec<i32>>, exclude: &Option<Vec<i32>>) -> bool {
    if let Some(include) = include {
        return include.contains(&code);
    }

    if let Some(exclude) = exclude {
        return !exclude.contains(&code);
    }

    true
}

fn build_parser(options: &CliOptions) -> PhotonParser {
    if options.unknown {
        PhotonParser::new(PhotonParserConfig::new(
            "live".to_string(),
            options.debug,
            true,
            WorldMap::from_embedded().unwrap_or_else(|_| WorldMap::empty()),
            ItemNameResolver::download_default().unwrap_or_else(|_| ItemNameResolver::empty()),
        ))
    } else {
        PhotonParser::new(PhotonParserConfig::with_defaults(
            "live".to_string(),
            options.debug,
        ))
    }
}

fn select_device(interface: Option<&str>) -> Result<Device, Box<dyn Error>> {
    if let Some(interface) = interface {
        return Device::list()?
            .into_iter()
            .find(|device| device.name == interface)
            .ok_or_else(|| format!("network interface not found: {interface}").into());
    }

    Device::lookup()?.ok_or_else(|| "no default network interface found".into())
}

fn build_capture_filter(options: &CliOptions) -> Result<CaptureFilter, Box<dyn Error>> {
    let mut filter = if options.any_port {
        CaptureFilter::any_port()
    } else {
        CaptureFilter::with_ports(options.ports.iter().copied())
    };

    if let Some(host_filter) = build_host_filter(options)? {
        filter.set_host_filter(host_filter);
    }

    Ok(filter)
}

fn build_host_filter(options: &CliOptions) -> Result<Option<HostFilter>, Box<dyn Error>> {
    if options.host_cidrs.is_empty() && options.hosts_files.is_empty() {
        return Ok(None);
    }

    if options.host_cidrs.is_empty() && options.hosts_files.len() == 1 {
        return Ok(Some(
            HostFilter::from_file(&options.hosts_files[0]).map_err(|error| error.0)?,
        ));
    }

    let mut cidrs = options.host_cidrs.clone();
    for hosts_file in &options.hosts_files {
        let content = fs::read_to_string(hosts_file)?;
        for line in content.lines() {
            let cidr = line.split('#').next().unwrap_or_default().trim();
            if !cidr.is_empty() {
                cidrs.push(cidr.to_string());
            }
        }
    }

    Ok(Some(HostFilter::from_cidrs(cidrs)?))
}

fn build_bpf_filter(options: &CliOptions) -> Option<String> {
    if options.any_port {
        return None;
    }

    Some(match options.ports.as_slice() {
        [] => "udp".to_string(),
        [port] => format!("udp and port {port}"),
        ports => {
            let ports = ports
                .iter()
                .map(|port| format!("port {port}"))
                .collect::<Vec<_>>()
                .join(" or ");
            format!("udp and ({ports})")
        }
    })
}

fn parse_args<I>(args: I) -> Result<CliOptions, Box<dyn Error>>
where
    I: IntoIterator<Item = String>,
{
    let mut options = CliOptions::default();
    let mut ports_overridden = false;
    let mut args = args.into_iter();

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--interface" => {
                options.interface = Some(next_value(&mut args, "--interface")?);
            }
            "--port" => {
                let port = next_value(&mut args, "--port")?
                    .parse::<u16>()
                    .map_err(|error| format!("invalid --port value: {error}"))?;
                if !ports_overridden {
                    options.ports.clear();
                    ports_overridden = true;
                }
                options.ports.push(port);
                options.any_port = false;
            }
            "--any-port" => {
                options.any_port = true;
                options.ports.clear();
                ports_overridden = true;
            }
            "--host-cidr" => {
                options
                    .host_cidrs
                    .push(next_value(&mut args, "--host-cidr")?);
            }
            "--hosts-file" => {
                options
                    .hosts_files
                    .push(PathBuf::from(next_value(&mut args, "--hosts-file")?));
            }
            "--debug" => {
                options.debug = true;
            }
            "--unknown" => {
                options.unknown = true;
            }
            "--count" => {
                options.count = Some(
                    next_value(&mut args, "--count")?
                        .parse::<usize>()
                        .map_err(|error| format!("invalid --count value: {error}"))?,
                );
            }
            "--no-events" => {
                options.no_events = true;
            }
            "--no-operations" => {
                options.no_operations = true;
            }
            "--op-include" => {
                options.op_include = Some(parse_code_list(&next_value(&mut args, "--op-include")?));
            }
            "--event-include" => {
                options.event_include =
                    Some(parse_code_list(&next_value(&mut args, "--event-include")?));
            }
            "--op-exclude" => {
                options.op_exclude = Some(parse_code_list(&next_value(&mut args, "--op-exclude")?));
            }
            "--event-exclude" => {
                options.event_exclude =
                    Some(parse_code_list(&next_value(&mut args, "--event-exclude")?));
            }
            _ => return Err(format!("unknown argument: {arg}").into()),
        }
    }

    if options.op_include.is_some() && options.op_exclude.is_some() {
        panic!("--op-include and --op-exclude cannot both be specified");
    }
    if options.event_include.is_some() && options.event_exclude.is_some() {
        panic!("--event-include and --event-exclude cannot both be specified");
    }

    Ok(options)
}

fn parse_code_list(value: &str) -> Vec<i32> {
    value
        .split(',')
        .map(str::trim)
        .filter(|code| !code.is_empty())
        .map(|code| {
            code.parse::<i32>()
                .unwrap_or_else(|error| panic!("invalid code {code:?}: {error}"))
        })
        .collect()
}

fn next_value<I>(args: &mut I, flag: &str) -> Result<String, Box<dyn Error>>
where
    I: Iterator<Item = String>,
{
    args.next()
        .ok_or_else(|| format!("missing value for {flag}").into())
}

fn print_usage() {
    println!(
        "\
Usage: cargo run --bin debug_packets -- [options]

Options:
  --interface <name>   Capture interface name; defaults to pcap's default device
  --port <port>        UDP port to capture; repeatable, defaults to 5056 and 4535
  --any-port           Capture any UDP port
  --host-cidr <cidr>   Host CIDR filter; repeatable
  --hosts-file <path>  File of host CIDRs, one per line with optional # comments
  --debug              Enable parser debug output
  --unknown            Emit unknown decoded packet records
  --count <n>          Stop after processing n matching UDP packets
  --no-events          Do not emit event packets
  --no-operations      Do not emit operation packets
  --op-include <list>  Emit only operation codes in a comma-separated integer list
  --event-include <list>
                       Emit only event codes in a comma-separated integer list
  --op-exclude <list>  Do not emit operation codes in a comma-separated integer list
  --event-exclude <list>
                       Do not emit event codes in a comma-separated integer list
  -h, --help           Show this help text"
    );
}

#[cfg(test)]
mod tests {
    use super::{CliOptions, build_bpf_filter, code_filter_matches, parse_args};
    use std::path::PathBuf;

    fn parse(args: &[&str]) -> CliOptions {
        parse_args(args.iter().map(|arg| arg.to_string())).unwrap()
    }

    #[test]
    fn defaults_select_albion_ports() {
        let options = parse(&[]);

        assert_eq!(options.ports, vec![5056, 4535]);
        assert!(!options.any_port);
    }

    #[test]
    fn repeated_ports_override_defaults() {
        let options = parse(&["--port", "6000", "--port", "7000"]);

        assert_eq!(options.ports, vec![6000, 7000]);
        assert!(!options.any_port);
    }

    #[test]
    fn any_port_clears_port_filtering() {
        let options = parse(&["--any-port"]);

        assert!(options.ports.is_empty());
        assert!(options.any_port);
    }

    #[test]
    fn host_cidr_and_hosts_file_are_both_accepted() {
        let options = parse(&[
            "--host-cidr",
            "192.168.1.0/24",
            "--hosts-file",
            "albion-hosts.txt",
        ]);

        assert_eq!(options.host_cidrs, vec!["192.168.1.0/24"]);
        assert_eq!(options.hosts_files, vec![PathBuf::from("albion-hosts.txt")]);
    }

    #[test]
    fn bpf_filter_uses_default_ports() {
        let options = parse(&[]);

        assert_eq!(
            build_bpf_filter(&options),
            Some("udp and (port 5056 or port 4535)".to_string())
        );
    }

    #[test]
    fn bpf_filter_is_absent_for_any_port() {
        let options = parse(&["--any-port"]);

        assert_eq!(build_bpf_filter(&options), None);
    }

    #[test]
    fn parses_output_filter_options() {
        let options = parse(&[
            "--no-events",
            "--no-operations",
            "--op-include",
            "83,174",
            "--event-exclude",
            "3,73",
        ]);

        assert!(options.no_events);
        assert!(options.no_operations);
        assert_eq!(options.op_include, Some(vec![83, 174]));
        assert_eq!(options.event_exclude, Some(vec![3, 73]));
    }

    #[test]
    fn include_filter_allows_only_listed_codes() {
        assert!(code_filter_matches(83, &Some(vec![83, 174]), &None));
        assert!(!code_filter_matches(82, &Some(vec![83, 174]), &None));
    }

    #[test]
    fn exclude_filter_rejects_listed_codes() {
        assert!(!code_filter_matches(83, &None, &Some(vec![83, 174])));
        assert!(code_filter_matches(82, &None, &Some(vec![83, 174])));
    }

    #[test]
    #[should_panic(expected = "--op-include and --op-exclude cannot both be specified")]
    fn panics_when_operation_include_and_exclude_are_both_set() {
        parse(&["--op-include", "83", "--op-exclude", "174"]);
    }

    #[test]
    #[should_panic(expected = "--event-include and --event-exclude cannot both be specified")]
    fn panics_when_event_include_and_exclude_are_both_set() {
        parse(&["--event-include", "3", "--event-exclude", "73"]);
    }
}
