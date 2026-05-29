use crate::{
    error::Result,
    event_codes::EventCode,
    extracted_packet::{ExtractedPacket, MarketPlaceNotification},
    models::{
        AlbionLocation, AlbionMail, CachedOrder, ChatChannel, MailInfoMetadata, OperationType,
        PlayerState, TradeType, WorldMap,
    },
    names,
    operation_codes::OperationCode,
    packet::{DecodedEvent, DecodedOperation, DecodedPacket, DecodedUnknown},
    protocol18::Protocol18Deserializer,
    requests::{
        auction_buy_offer::AuctionBuyOffer, auction_get_offers::AuctionGetOffers,
        auction_get_requests::AuctionGetRequests,
        auction_sell_specific_item::AuctionSellSpecificItem as AuctionSellSpecificItemRequest,
    },
    responses::{
        AuctionGetOffersResult, AuctionGetRequestsResult, AuctionTrade, AuctionTradeResponse,
        ChatMessage, GetMailInfos, JoinResponse, JoinedChatChannel, LeftChatChannel, ReadMail,
    },
    util::{params_to_json, read_i32_be, to_signed_short, value_i64},
};
use serde_json::{json, Value};
use std::{
    collections::{BTreeMap, HashMap},
    sync::Arc,
};

const COMMAND_DISCONNECT: u8 = 4;
const COMMAND_SEND_RELIABLE: u8 = 6;
const COMMAND_SEND_UNRELIABLE: u8 = 7;
const COMMAND_SEND_FRAGMENT: u8 = 8;

const MESSAGE_OPERATION_REQUEST: u8 = 2;
const MESSAGE_OPERATION_RESPONSE: u8 = 3;
const MESSAGE_EVENT: u8 = 4;
const ITEM_NAME_MAPPINGS_URL: &str =
    "https://cdn.albionfreemarket.com/AlbionFormattedItemsParser/us_name_mappings.json";

struct PendingSegment {
    payload: Vec<u8>,
    written: usize,
    total_length: usize,
}

pub struct PhotonParser {
    file_name: String,
    debug: bool,
    world_map: Arc<WorldMap>,
    deserializer: Protocol18Deserializer,
    pending_segments: HashMap<i32, PendingSegment>,
    decoded_packets: Vec<DecodedPacket>,
    capture_unknown_packets: bool,
    market_orders_by_id: HashMap<i64, CachedOrder>,
    unconfirmed_trade: Option<AuctionTrade>,
    mail_infos_by_id: HashMap<i64, MailInfoMetadata>,
    read_mails_by_id: HashMap<i64, ReadMail>,
    albion_mails_by_id: HashMap<i64, AlbionMail>,
    item_names_by_id: HashMap<String, String>,
    chat_channels_by_id: HashMap<i64, ChatChannel>,
    player_state: PlayerState,
}

impl PhotonParser {
    pub fn new(file_name: String, debug: bool) -> Self {
        let world_map = WorldMap::from_embedded().unwrap_or_else(|_| WorldMap::empty());
        Self::with_world_map(file_name, debug, world_map)
    }

    pub fn with_world_map(file_name: String, debug: bool, world_map: WorldMap) -> Self {
        Self::with_world_map_and_item_names(file_name, debug, world_map, download_item_names())
    }

    fn with_world_map_and_item_names(
        file_name: String,
        debug: bool,
        world_map: WorldMap,
        item_names_by_id: HashMap<String, String>,
    ) -> Self {
        let world_map = Arc::new(world_map);
        Self {
            file_name,
            debug,
            world_map: Arc::clone(&world_map),
            deserializer: Protocol18Deserializer,
            pending_segments: HashMap::new(),
            decoded_packets: Vec::new(),
            capture_unknown_packets: std::env::var("ALBION_NETWORK_DEBUG").as_deref() == Ok("1"),
            market_orders_by_id: HashMap::new(),
            unconfirmed_trade: None,
            mail_infos_by_id: HashMap::new(),
            read_mails_by_id: HashMap::new(),
            albion_mails_by_id: HashMap::new(),
            item_names_by_id,
            chat_channels_by_id: HashMap::new(),
            player_state: PlayerState::new(world_map),
        }
    }

    pub fn decoded_packets(&self) -> &[DecodedPacket] {
        &self.decoded_packets
    }

    pub fn market_order_count(&self) -> usize {
        self.market_orders_by_id.len()
    }

    pub fn player_state(&self) -> &PlayerState {
        &self.player_state
    }

    pub fn player_state_mut(&mut self) -> &mut PlayerState {
        &mut self.player_state
    }

    pub fn albion_mails(&self) -> &HashMap<i64, AlbionMail> {
        &self.albion_mails_by_id
    }

    pub fn into_decoded_packets(self) -> Vec<DecodedPacket> {
        self.decoded_packets
    }

    pub fn receive_packet(
        &mut self,
        payload: &[u8],
        packet_number: usize,
        source: &str,
        destination: &str,
    ) -> Result<&'static str> {
        if payload.len() < 12 {
            return Ok("InvalidHeader");
        }
        let flags = payload[2];
        let command_count = payload[3];
        if flags == 1 {
            self.player_state.set_has_encrypted_data(true);
            return Ok("Encrypted");
        }
        let mut offset = 12;
        let mut status = "Undefined";
        for command_index in 0..command_count {
            if payload.len().saturating_sub(offset) < 12 {
                return Ok("InvalidHeader");
            }
            let result = self.handle_command(
                payload,
                offset,
                packet_number,
                command_index,
                source,
                destination,
            )?;
            status = result.0;
            offset = result.1;
            if status == "InvalidHeader" {
                return Ok(status);
            }
        }
        Ok(status)
    }

    fn handle_command(
        &mut self,
        data: &[u8],
        mut offset: usize,
        packet_number: usize,
        command_index: u8,
        source: &str,
        destination: &str,
    ) -> Result<(&'static str, usize)> {
        let command_type = data[offset];
        let command_length = read_i32_be(data, offset + 4)? - 12;
        let sequence_number = read_i32_be(data, offset + 8)?;
        offset += 12;
        if self.debug {
            eprintln!(
                "DEBUG:albion:packet={packet_number} command={command_index} type={command_type} sequence={sequence_number} payload_length={command_length}"
            );
        }
        if command_length < 0 || data.len().saturating_sub(offset) < command_length as usize {
            return Ok(("InvalidHeader", offset));
        }
        let command_length = command_length as usize;
        match command_type {
            COMMAND_DISCONNECT => Ok(("DisconnectCommand", offset + command_length)),
            COMMAND_SEND_UNRELIABLE => {
                if command_length < 4 {
                    return Ok(("InvalidHeader", offset));
                }
                self.handle_send_reliable(
                    data,
                    offset + 4,
                    command_length - 4,
                    packet_number,
                    source,
                    destination,
                )
            }
            COMMAND_SEND_RELIABLE => self.handle_send_reliable(
                data,
                offset,
                command_length,
                packet_number,
                source,
                destination,
            ),
            COMMAND_SEND_FRAGMENT => self.handle_send_fragment(
                data,
                offset,
                command_length,
                packet_number,
                source,
                destination,
            ),
            _ => Ok(("Undefined", offset + command_length)),
        }
    }

    fn handle_send_reliable(
        &mut self,
        data: &[u8],
        offset: usize,
        command_length: usize,
        packet_number: usize,
        source: &str,
        destination: &str,
    ) -> Result<(&'static str, usize)> {
        if command_length < 2 || data.len().saturating_sub(offset) < command_length {
            return Ok(("InvalidHeader", offset));
        }
        let message_type = data[offset + 1];
        let operation_payload = &data[offset + 2..offset + command_length];
        if message_type == 131 {
            self.player_state.set_has_encrypted_data(true);
            return Ok(("Encrypted", offset + command_length));
        }
        match message_type {
            MESSAGE_OPERATION_REQUEST => {
                let (_, params) = self
                    .deserializer
                    .deserialize_operation_request(operation_payload)?;
                self.record_operation(
                    "request",
                    params,
                    None,
                    "",
                    packet_number,
                    source,
                    destination,
                )?;
            }
            MESSAGE_OPERATION_RESPONSE => {
                let (_, return_code, debug_message, params) = self
                    .deserializer
                    .deserialize_operation_response(operation_payload)?;
                self.record_operation(
                    "response",
                    params,
                    Some(return_code),
                    &debug_message,
                    packet_number,
                    source,
                    destination,
                )?;
            }
            MESSAGE_EVENT => {
                let (event_code, params) = self
                    .deserializer
                    .deserialize_event_data(operation_payload)?;
                self.record_event(event_code, params, packet_number, source, destination)?;
            }
            _ => {}
        }
        Ok(("Success", offset + command_length))
    }

    fn handle_send_fragment(
        &mut self,
        data: &[u8],
        mut offset: usize,
        command_length: usize,
        packet_number: usize,
        source: &str,
        destination: &str,
    ) -> Result<(&'static str, usize)> {
        if command_length < 20 || data.len().saturating_sub(offset) < command_length {
            return Ok(("InvalidHeader", offset));
        }
        let start_sequence_number = read_i32_be(data, offset)?;
        let total_length = read_i32_be(data, offset + 12)? as usize;
        let fragment_offset = read_i32_be(data, offset + 16)? as usize;
        offset += 20;
        let fragment_length = command_length - 20;
        let fragment = &data[offset..offset + fragment_length];

        let pending = self
            .pending_segments
            .entry(start_sequence_number)
            .or_insert_with(|| PendingSegment {
                payload: vec![0; total_length],
                written: 0,
                total_length,
            });
        pending.payload[fragment_offset..fragment_offset + fragment_length]
            .copy_from_slice(fragment);
        pending.written += fragment_length;

        if pending.written >= pending.total_length {
            let total_payload = self
                .pending_segments
                .remove(&start_sequence_number)
                .unwrap()
                .payload;
            let (status, _) = self.handle_send_reliable(
                &total_payload,
                0,
                total_payload.len(),
                packet_number,
                source,
                destination,
            )?;
            return Ok((status, offset + fragment_length));
        }

        Ok(("Success", offset + fragment_length))
    }

    fn record_operation(
        &mut self,
        packet_kind: &str,
        parameters: BTreeMap<u8, Value>,
        return_code: Option<i16>,
        debug_message: &str,
        packet_number: usize,
        source: &str,
        destination: &str,
    ) -> Result<()> {
        let operation_code = match parse_operation_code(&parameters) {
            Ok(operation_code) => operation_code,
            Err(error) => {
                if self.capture_unknown_packets {
                    self.record_unknown(
                        format!("operation_{packet_kind}"),
                        format!("operation_{packet_kind}"),
                        253,
                        error.raw_code,
                        error.reason,
                        parameters,
                        return_code,
                        debug_message,
                        packet_number,
                        source,
                        destination,
                    );
                    return Ok(());
                }
                return Err(error.message.into());
            }
        };
        let operation_name = operation_code.name();
        let extracted =
            self.extract_operation(packet_kind, operation_code, &parameters, return_code);
        self.decoded_packets
            .push(DecodedPacket::Operation(DecodedOperation {
                file: self.file_name.clone(),
                packet_number,
                direction: direction(source, destination).to_string(),
                source: source.to_string(),
                destination: destination.to_string(),
                message_type: format!("operation_{packet_kind}"),
                code: operation_code,
                name: operation_name.to_string(),
                return_code,
                debug_message: debug_message.to_string(),
                parameters: params_to_json(&parameters),
                extracted,
            }));
        Ok(())
    }

    fn record_event(
        &mut self,
        photon_event_code: u8,
        mut parameters: BTreeMap<u8, Value>,
        packet_number: usize,
        source: &str,
        destination: &str,
    ) -> Result<()> {
        if photon_event_code == EventCode::Move as u8 {
            parameters.insert(252, json!(EventCode::Move as i32));
        }
        let event_code = match parse_event_code(&parameters) {
            Ok(event_code) => event_code,
            Err(error) => {
                if self.capture_unknown_packets {
                    self.record_unknown(
                        "event".to_string(),
                        "event".to_string(),
                        252,
                        error.raw_code,
                        error.reason,
                        parameters,
                        None,
                        "",
                        packet_number,
                        source,
                        destination,
                    );
                    return Ok(());
                }
                return Err(error.message.into());
            }
        };
        let event_name = event_code.name();
        let extracted = self.extract_event(event_code, &parameters);
        self.decoded_packets
            .push(DecodedPacket::Event(DecodedEvent {
                file: self.file_name.clone(),
                packet_number,
                direction: direction(source, destination).to_string(),
                source: source.to_string(),
                destination: destination.to_string(),
                message_type: "event".to_string(),
                code: event_code,
                name: event_name.to_string(),
                return_code: None,
                debug_message: String::new(),
                parameters: params_to_json(&parameters),
                extracted,
            }));
        Ok(())
    }

    fn record_unknown(
        &mut self,
        message_type: String,
        kind: String,
        code_parameter: u8,
        raw_code: Option<i32>,
        reason: &'static str,
        parameters: BTreeMap<u8, Value>,
        return_code: Option<i16>,
        debug_message: &str,
        packet_number: usize,
        source: &str,
        destination: &str,
    ) {
        self.decoded_packets
            .push(DecodedPacket::Unknown(DecodedUnknown {
                file: self.file_name.clone(),
                packet_number,
                direction: direction(source, destination).to_string(),
                source: source.to_string(),
                destination: destination.to_string(),
                message_type,
                kind,
                code_parameter,
                raw_code,
                reason: reason.to_string(),
                return_code,
                debug_message: debug_message.to_string(),
                parameters: params_to_json(&parameters),
            }));
    }

    fn extract_operation(
        &mut self,
        packet_kind: &str,
        operation_code: OperationCode,
        parameters: &BTreeMap<u8, Value>,
        return_code: Option<i16>,
    ) -> Option<ExtractedPacket> {
        let fallback_location_index = self.player_state.location_index();
        match (operation_code, packet_kind) {
            (OperationCode::AuctionGetOffers, "request") => {
                let orders = self.extract_market_orders(parameters, fallback_location_index);
                return Some(ExtractedPacket::AuctionGetOffersRequest(AuctionGetOffers {
                    market_order_count: orders.len(),
                    market_orders: orders,
                }));
            }
            (OperationCode::AuctionGetRequests, "request") => {
                let orders = self.extract_market_orders(parameters, fallback_location_index);
                return Some(ExtractedPacket::AuctionGetRequestsRequest(
                    AuctionGetRequests {
                        market_order_count: orders.len(),
                        market_orders: orders,
                    },
                ));
            }
            (OperationCode::AuctionGetOffers, "response") => {
                let orders = self.extract_market_orders(parameters, fallback_location_index);
                for order in &orders {
                    self.market_orders_by_id.insert(order.id, order.clone());
                }
                return Some(ExtractedPacket::AuctionGetOffersResponse(
                    AuctionGetOffersResult {
                        market_order_count: orders.len(),
                        market_orders: orders,
                    },
                ));
            }
            (OperationCode::AuctionGetRequests, "response") => {
                let orders = self.extract_market_orders(parameters, fallback_location_index);
                for order in &orders {
                    self.market_orders_by_id.insert(order.id, order.clone());
                }
                return Some(ExtractedPacket::AuctionGetRequestsResponse(
                    AuctionGetRequestsResult {
                        market_order_count: orders.len(),
                        market_orders: orders,
                    },
                ));
            }
            (OperationCode::AuctionBuyOffer, "request") => {
                let amount = parameters.get(&1).and_then(value_i64);
                let order_id = parameters.get(&2).and_then(value_i64);
                let cached_order =
                    order_id.and_then(|id| self.market_orders_by_id.get(&id).cloned());
                let request = AuctionBuyOffer {
                    amount,
                    cached_order: cached_order.clone(),
                    order_id,
                };
                self.unconfirmed_trade = order_id.map(|id| AuctionTrade {
                    amount,
                    silver_amount: silver_amount(amount, cached_order.as_ref()),
                    operation: operation_from_cached_order(
                        cached_order.as_ref(),
                        &TradeType::Instant,
                    ),
                    trade_type: TradeType::Instant,
                    order: cached_order,
                    id,
                });
                return Some(ExtractedPacket::AuctionBuyOfferRequest(request));
            }
            (OperationCode::AuctionSellSpecificItem, "request") => {
                let amount = parameters.get(&4).and_then(value_i64);
                let order_id = parameters.get(&1).and_then(value_i64);
                let cached_order =
                    order_id.and_then(|id| self.market_orders_by_id.get(&id).cloned());
                let request = AuctionSellSpecificItemRequest {
                    amount,
                    cached_order: cached_order.clone(),
                    order_id,
                };
                self.unconfirmed_trade = order_id.map(|id| AuctionTrade {
                    amount,
                    silver_amount: silver_amount(amount, cached_order.as_ref()),
                    operation: operation_from_cached_order(
                        cached_order.as_ref(),
                        &TradeType::Instant,
                    ),
                    trade_type: TradeType::Instant,
                    order: cached_order,
                    id,
                });
                return Some(ExtractedPacket::AuctionSellSpecificItemRequest(request));
            }
            (
                OperationCode::AuctionBuyOffer | OperationCode::AuctionSellSpecificItem,
                "response",
            ) => {
                let success = return_code == Some(0);
                let response = AuctionTradeResponse {
                    confirmed_trade: success.then(|| self.unconfirmed_trade.clone()).flatten(),
                    success,
                };
                self.unconfirmed_trade = None;
                return Some(ExtractedPacket::AuctionTradeResponse(response));
            }
            (OperationCode::Join, "response") => {
                let response = JoinResponse::from_params(parameters);
                self.player_state
                    .set_user_object_id(response.user_object_id);
                if let Some(player_name) = response.player_name.as_deref() {
                    self.player_state.set_player_name(player_name);
                }
                self.player_state
                    .set_location_raw(&response.player_location);
                return Some(ExtractedPacket::JoinResponse(response));
            }
            (OperationCode::GetMailInfos, "response") => {
                let response = GetMailInfos::from_params(parameters);
                self.cache_mail_infos(&response);
                return Some(ExtractedPacket::GetMailInfos(response));
            }
            (OperationCode::ReadMail, "response") => {
                let response = ReadMail::from_params(parameters);
                return self
                    .cache_read_mail(response)
                    .map(ExtractedPacket::AlbionMail);
            }
            _ => {}
        }

        None
    }

    fn cache_mail_infos(&mut self, response: &GetMailInfos) {
        for index in 0..response.mail_ids.len() {
            let Some(location_id) = response
                .location_ids
                .get(index)
                .map(|location_id| normalize_mail_location_id(location_id))
            else {
                continue;
            };
            let Some(info_type) = response.types.get(index).copied() else {
                continue;
            };
            let Some(received) = response.received.get(index).copied() else {
                continue;
            };

            let metadata = MailInfoMetadata {
                mail_id: response.mail_ids[index],
                location_id,
                info_type,
                received,
            };
            self.mail_infos_by_id
                .insert(metadata.mail_id, metadata.clone());

            if let Some(read_mail) = self.read_mails_by_id.get(&metadata.mail_id).cloned() {
                let mail = self.build_albion_mail(&metadata, &read_mail);
                self.albion_mails_by_id.insert(mail.id, mail);
            }
        }
    }

    fn cache_read_mail(&mut self, response: ReadMail) -> Option<AlbionMail> {
        self.read_mails_by_id
            .insert(response.mail_id, response.clone());
        let metadata = self.mail_infos_by_id.get(&response.mail_id)?.clone();
        let mail = self.build_albion_mail(&metadata, &response);
        self.albion_mails_by_id.insert(mail.id, mail.clone());
        Some(mail)
    }

    fn build_albion_mail(&self, metadata: &MailInfoMetadata, read_mail: &ReadMail) -> AlbionMail {
        let mut mail = AlbionMail::from_correlated(
            metadata.mail_id,
            self.world_map.resolve_location(&metadata.location_id),
            self.player_state.player_name.clone(),
            metadata.info_type,
            metadata.received,
            &read_mail.mail_string,
        );
        mail.item_name = self.item_names_by_id.get(&mail.item_id).cloned();
        mail
    }

    fn extract_market_orders(
        &self,
        params: &BTreeMap<u8, Value>,
        fallback_location_index: Option<&str>,
    ) -> Vec<CachedOrder> {
        let Some(raw_orders) = params.get(&0) else {
            return Vec::new();
        };
        let values: Vec<Value> = match raw_orders {
            Value::Array(items) => items.clone(),
            item => vec![item.clone()],
        };
        values
            .into_iter()
            .filter_map(|value| match value {
                Value::String(text) => serde_json::from_str(&text).ok(),
                Value::Object(_) => serde_json::from_value(value).ok(),
                _ => None,
            })
            .map(|mut order: CachedOrder| {
                order.location = if order.location.is_unknown() {
                    fallback_location_index
                        .map(|location| self.world_map.resolve_location(location))
                        .unwrap_or_else(AlbionLocation::unknown)
                } else {
                    self.world_map.resolve_location(&order.location.id)
                };
                order.item_name = self.item_names_by_id.get(&order.item_id).cloned();
                order
            })
            .collect()
    }

    fn extract_event(
        &mut self,
        event_code: EventCode,
        parameters: &BTreeMap<u8, Value>,
    ) -> Option<ExtractedPacket> {
        match event_code {
            EventCode::MarketPlaceNotification => Some(ExtractedPacket::MarketPlaceNotification(
                MarketPlaceNotification {
                    notification: parameters.get(&0).cloned().unwrap_or(Value::Null),
                },
            )),
            EventCode::ChatMessage => {
                let channel_type = parameters
                    .get(&0)
                    .and_then(value_i64)
                    .and_then(|channel_id| self.chat_channels_by_id.get(&channel_id).copied());
                Some(ExtractedPacket::ChatMessage(
                    ChatMessage::from_params_with_channel_type(parameters, channel_type),
                ))
            }
            EventCode::ChatSay => Some(ExtractedPacket::ChatMessage(ChatMessage::from_say_params(
                parameters,
            ))),
            EventCode::JoinedChatChannel => {
                let response = JoinedChatChannel::from_params(parameters);
                self.chat_channels_by_id.insert(
                    response.channel_id,
                    ChatChannel::from_chat_index(i64::from(response.chat_index)),
                );
                Some(ExtractedPacket::JoinedChatChannel(response))
            }
            EventCode::LeftChatChannel => {
                let response = LeftChatChannel::from_params(parameters);
                self.chat_channels_by_id.remove(&response.channel_id);
                Some(ExtractedPacket::LeftChatChannel(response))
            }
            _ => None,
        }
    }
}

struct CodeParseError {
    raw_code: Option<i32>,
    reason: &'static str,
    message: String,
}

fn parse_operation_code(
    params: &BTreeMap<u8, Value>,
) -> std::result::Result<OperationCode, CodeParseError> {
    let Some(value) = params.get(&253).and_then(value_i64) else {
        return Err(CodeParseError {
            raw_code: None,
            reason: "missing_operation_code",
            message: "Operation code parameter 253 is missing".to_string(),
        });
    };
    let code = to_signed_short(value);
    names::operation(code).ok_or_else(|| CodeParseError {
        raw_code: Some(code),
        reason: "unknown_operation_code",
        message: format!("Unknown operation code in parameter 253: {code}"),
    })
}

fn parse_event_code(
    params: &BTreeMap<u8, Value>,
) -> std::result::Result<EventCode, CodeParseError> {
    let Some(value) = params.get(&252).and_then(value_i64) else {
        return Err(CodeParseError {
            raw_code: None,
            reason: "missing_event_code",
            message: "Event code parameter 252 is missing".to_string(),
        });
    };
    let code = to_signed_short(value);
    if let Some(event_code) = names::event(code) {
        return Ok(event_code);
    }
    let unsigned_value = (code as i64 & 0xffff) as i32;
    let shifted = unsigned_value >> 4;
    if (unsigned_value & 0x0f) == 0x01 {
        if let Some(event_code) = names::event(shifted) {
            return Ok(event_code);
        }
    }
    Err(CodeParseError {
        raw_code: Some(code),
        reason: "unknown_event_code",
        message: format!("Unknown event code in parameter 252: {code}"),
    })
}

fn operation_from_cached_order(
    cached_order: Option<&CachedOrder>,
    trade_type: &TradeType,
) -> OperationType {
    cached_order
        .map(|order| OperationType::from_auction_type(&order.auction_type, trade_type))
        .unwrap_or_else(|| OperationType::Unknown("missing_cached_order".to_string()))
}

fn silver_amount(amount: Option<i64>, cached_order: Option<&CachedOrder>) -> Option<i64> {
    let amount = amount?;
    let order = cached_order?;
    Some(
        (((order.unit_price_silver * amount) - order.distance_fee) as f64 / 10_000.0).floor()
            as i64,
    )
}

fn direction(source: &str, destination: &str) -> &'static str {
    if source.ends_with(":5056") {
        "server_to_client"
    } else if destination.ends_with(":5056") {
        "client_to_server"
    } else {
        "unknown"
    }
}

fn normalize_mail_location_id(location_id: &str) -> String {
    if location_id == "@BLACK_MARKET" {
        return "3003".to_string();
    }
    location_id
        .split('@')
        .nth(1)
        .unwrap_or(location_id)
        .to_string()
}

fn download_item_names() -> HashMap<String, String> {
    let response = ureq::get(ITEM_NAME_MAPPINGS_URL)
        .call()
        .expect("failed to download item name mappings");
    let text = response
        .into_string()
        .expect("failed to read item name mappings response");
    parse_item_name_mappings(&text).expect("failed to parse item name mappings")
}

fn parse_item_name_mappings(json: &str) -> serde_json::Result<HashMap<String, String>> {
    serde_json::from_str(json)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{AlbionLocation, AuctionType, MailInfoType};
    use serde_json::json;

    fn parser() -> PhotonParser {
        PhotonParser::with_world_map_and_item_names(
            "test".to_string(),
            false,
            WorldMap::from_embedded().unwrap(),
            item_names(),
        )
    }

    fn parser_with_unknown_capture() -> PhotonParser {
        let mut parser = parser();
        parser.capture_unknown_packets = true;
        parser
    }

    fn item_names() -> HashMap<String, String> {
        HashMap::from([
            ("T1_HIDE".to_string(), "Scraps of Hide".to_string()),
            ("T4_HIDE".to_string(), "Adept's Hide".to_string()),
        ])
    }

    #[test]
    fn parses_item_name_mappings() {
        let mappings =
            parse_item_name_mappings(r#"{"T1_HIDE":"Scraps of Hide","T4_HIDE":"Adept's Hide"}"#)
                .unwrap();

        assert_eq!(
            mappings.get("T1_HIDE").map(String::as_str),
            Some("Scraps of Hide")
        );
        assert_eq!(
            mappings.get("T4_HIDE").map(String::as_str),
            Some("Adept's Hide")
        );
    }

    #[test]
    fn invalid_item_name_mappings_return_parse_error() {
        assert!(parse_item_name_mappings("not json").is_err());
    }

    #[test]
    fn unknown_operation_code_errors_when_debug_capture_is_disabled() {
        let mut parser = parser();
        let mut params = BTreeMap::new();
        params.insert(253, json!(32_767));

        let error = parser
            .record_operation("request", params, None, "", 1, "client:1", "server:5056")
            .unwrap_err();

        assert_eq!(error.0, "Unknown operation code in parameter 253: 32767");
        assert!(parser.decoded_packets().is_empty());
    }

    #[test]
    fn unknown_operation_code_is_captured_when_debug_capture_is_enabled() {
        let mut parser = parser_with_unknown_capture();
        let mut params = BTreeMap::new();
        params.insert(1, json!("payload"));
        params.insert(253, json!(32_767));

        parser
            .record_operation("request", params, None, "", 1, "client:1", "server:5056")
            .unwrap();

        let DecodedPacket::Unknown(packet) = &parser.decoded_packets()[0] else {
            panic!("expected unknown packet");
        };

        assert_eq!(packet.message_type, "operation_request");
        assert_eq!(packet.kind, "operation_request");
        assert_eq!(packet.code_parameter, 253);
        assert_eq!(packet.raw_code, Some(32_767));
        assert_eq!(packet.reason, "unknown_operation_code");
        assert_eq!(packet.direction, "client_to_server");
        assert_eq!(packet.parameters.get("1"), Some(&json!("payload")));
    }

    #[test]
    fn unknown_event_code_is_captured_when_debug_capture_is_enabled() {
        let mut parser = parser_with_unknown_capture();
        let mut params = BTreeMap::new();
        params.insert(0, json!("payload"));
        params.insert(252, json!(32_767));

        parser
            .record_event(0, params, 1, "server:5056", "client:1")
            .unwrap();

        let DecodedPacket::Unknown(packet) = &parser.decoded_packets()[0] else {
            panic!("expected unknown packet");
        };

        assert_eq!(packet.message_type, "event");
        assert_eq!(packet.kind, "event");
        assert_eq!(packet.code_parameter, 252);
        assert_eq!(packet.raw_code, Some(32_767));
        assert_eq!(packet.reason, "unknown_event_code");
        assert_eq!(packet.direction, "server_to_client");
        assert_eq!(packet.parameters.get("0"), Some(&json!("payload")));
    }

    #[test]
    fn missing_codes_are_captured_without_raw_code_when_debug_capture_is_enabled() {
        let mut parser = parser_with_unknown_capture();

        parser
            .record_operation(
                "response",
                BTreeMap::new(),
                Some(0),
                "debug",
                1,
                "server:5056",
                "client:1",
            )
            .unwrap();
        parser
            .record_event(0, BTreeMap::new(), 2, "server:5056", "client:1")
            .unwrap();

        let DecodedPacket::Unknown(operation) = &parser.decoded_packets()[0] else {
            panic!("expected unknown operation packet");
        };
        assert_eq!(operation.kind, "operation_response");
        assert_eq!(operation.code_parameter, 253);
        assert_eq!(operation.raw_code, None);
        assert_eq!(operation.reason, "missing_operation_code");
        assert_eq!(operation.return_code, Some(0));
        assert_eq!(operation.debug_message, "debug");

        let DecodedPacket::Unknown(event) = &parser.decoded_packets()[1] else {
            panic!("expected unknown event packet");
        };
        assert_eq!(event.kind, "event");
        assert_eq!(event.code_parameter, 252);
        assert_eq!(event.raw_code, None);
        assert_eq!(event.reason, "missing_event_code");
    }

    #[test]
    fn join_response_updates_player_state() {
        let mut parser = parser();
        let mut params = BTreeMap::new();
        params.insert(0, json!(123));
        params.insert(2, json!("PlayerOne"));
        params.insert(8, json!("Bridgewatch"));

        let extracted = parser
            .extract_operation("response", OperationCode::Join, &params, Some(0))
            .unwrap();

        let ExtractedPacket::JoinResponse(response) = extracted else {
            panic!("expected join response");
        };

        assert_eq!(response.player_name.as_deref(), Some("PlayerOne"));
        assert_eq!(parser.player_state().user_object_id(), Some(123));
        assert_eq!(parser.player_state().player_name, "PlayerOne");
        assert_eq!(
            parser.player_state().location.friendly_name(),
            "Bridgewatch"
        );
        assert_eq!(parser.player_state().location.location_id(), Some(2000));
    }

    #[test]
    fn market_orders_inherit_missing_location_id_from_player_state() {
        let mut parser = parser();
        parser.player_state_mut().set_location_raw("Bridgewatch");

        let mut params = BTreeMap::new();
        params.insert(
            0,
            json!([
                {
                    "Amount": 1,
                    "AuctionType": "offer",
                    "BuyerCharacterId": null,
                    "BuyerName": null,
                    "DistanceFee": 0,
                    "EnchantmentLevel": 0,
                    "Expires": "2026-06-25T07:55:20.513833",
                    "HasBuyerFetched": false,
                    "HasSellerFetched": false,
                    "Id": 14990497605_i64,
                    "IsFinished": false,
                    "ItemGroupTypeId": "T1_HIDE",
                    "ItemTypeId": "T1_HIDE",
                    "LocationId": null,
                    "QualityLevel": 1,
                    "ReferenceId": "7bf5e58d-b835-4969-acba-297bf80ec287",
                    "SellerCharacterId": null,
                    "SellerName": null,
                    "Tier": 1,
                    "TotalPriceSilver": 500000,
                    "UnitPriceSilver": 50000
                },
                {
                    "Amount": 1,
                    "AuctionType": "offer",
                    "BuyerCharacterId": null,
                    "BuyerName": null,
                    "DistanceFee": 0,
                    "EnchantmentLevel": 0,
                    "Expires": "2026-06-25T07:55:20.513833",
                    "HasBuyerFetched": false,
                    "HasSellerFetched": false,
                    "Id": 14990497606_i64,
                    "IsFinished": false,
                    "ItemGroupTypeId": "T1_HIDE",
                    "ItemTypeId": "T1_HIDE",
                    "LocationId": "9999",
                    "QualityLevel": 1,
                    "ReferenceId": "7bf5e58d-b835-4969-acba-297bf80ec288",
                    "SellerCharacterId": null,
                    "SellerName": null,
                    "Tier": 1,
                    "TotalPriceSilver": 500000,
                    "UnitPriceSilver": 50000
                }
            ]),
        );

        let extracted = parser
            .extract_operation(
                "response",
                OperationCode::AuctionGetOffers,
                &params,
                Some(0),
            )
            .unwrap();

        let ExtractedPacket::AuctionGetOffersResponse(response) = extracted else {
            panic!("expected auction get offers response");
        };

        assert_eq!(
            response.market_orders[0].location,
            AlbionLocation::with_names("2000", "Bridgewatch", "Bridgewatch")
        );
        assert_eq!(
            response.market_orders[1].location,
            AlbionLocation::unknown()
        );
    }

    #[test]
    fn market_orders_resolve_location_names_from_location_id() {
        let mut parser = parser();

        let mut params = market_order_params(14978117778);
        params.insert(
            0,
            json!([
                {
                    "Amount": 2,
                    "AuctionType": "offer",
                    "BuyerCharacterId": null,
                    "BuyerName": null,
                    "DistanceFee": 0,
                    "EnchantmentLevel": 0,
                    "Expires": "2026-06-22T03:34:16.096699",
                    "HasBuyerFetched": false,
                    "HasSellerFetched": false,
                    "Id": 14978117778_i64,
                    "IsFinished": false,
                    "ItemGroupTypeId": "T1_HIDE",
                    "ItemTypeId": "T1_HIDE",
                    "LocationId": "3008",
                    "QualityLevel": 1,
                    "ReferenceId": "7ae09894-4883-479b-932e-ff7914c82855",
                    "SellerCharacterId": "07b8fbc0-c512-4054-bc53-12312af94df3",
                    "SellerName": "CoelhoMalvado",
                    "Tier": 1,
                    "TotalPriceSilver": 100000,
                    "UnitPriceSilver": 50000
                }
            ]),
        );

        let extracted = parser
            .extract_operation(
                "response",
                OperationCode::AuctionGetOffers,
                &params,
                Some(0),
            )
            .unwrap();

        let ExtractedPacket::AuctionGetOffersResponse(response) = extracted else {
            panic!("expected auction get offers response");
        };

        assert_eq!(
            response.market_orders[0].location,
            AlbionLocation::with_names("3008", "Martlock Market", "Martlock Market")
        );
        assert_eq!(
            response.market_orders[0].item_name.as_deref(),
            Some("Scraps of Hide")
        );
    }

    #[test]
    fn market_orders_unknown_non_numeric_location_and_unknown_item_become_unknowns() {
        let mut parser = parser();
        let mut params = market_order_params(14978117778);
        params.insert(
            0,
            json!([
                {
                    "Amount": 2,
                    "AuctionType": "offer",
                    "BuyerCharacterId": null,
                    "BuyerName": null,
                    "DistanceFee": 0,
                    "EnchantmentLevel": 0,
                    "Expires": "2026-06-22T03:34:16.096699",
                    "HasBuyerFetched": false,
                    "HasSellerFetched": false,
                    "Id": 14978117778_i64,
                    "IsFinished": false,
                    "ItemGroupTypeId": "UNKNOWN_ITEM",
                    "ItemTypeId": "UNKNOWN_ITEM",
                    "LocationId": "NOT-A-WORLD-INDEX",
                    "QualityLevel": 1,
                    "ReferenceId": "7ae09894-4883-479b-932e-ff7914c82855",
                    "SellerCharacterId": "07b8fbc0-c512-4054-bc53-12312af94df3",
                    "SellerName": "CoelhoMalvado",
                    "Tier": 1,
                    "TotalPriceSilver": 100000,
                    "UnitPriceSilver": 50000
                }
            ]),
        );

        let extracted = parser
            .extract_operation(
                "response",
                OperationCode::AuctionGetOffers,
                &params,
                Some(0),
            )
            .unwrap();

        let ExtractedPacket::AuctionGetOffersResponse(response) = extracted else {
            panic!("expected auction get offers response");
        };

        assert_eq!(
            response.market_orders[0].location,
            AlbionLocation::unknown()
        );
        assert_eq!(response.market_orders[0].item_name, None);
    }

    #[test]
    fn auction_operations_return_typed_extracted_packets() {
        let mut parser = parser();
        let params = BTreeMap::new();

        let extracted = parser
            .extract_operation("request", OperationCode::AuctionGetOffers, &params, None)
            .unwrap();
        assert!(matches!(
            extracted,
            ExtractedPacket::AuctionGetOffersRequest(_)
        ));

        let extracted = parser
            .extract_operation("request", OperationCode::AuctionGetRequests, &params, None)
            .unwrap();
        assert!(matches!(
            extracted,
            ExtractedPacket::AuctionGetRequestsRequest(_)
        ));

        let extracted = parser
            .extract_operation(
                "response",
                OperationCode::AuctionGetRequests,
                &market_order_params(14978117778),
                Some(0),
            )
            .unwrap();
        assert!(matches!(
            extracted,
            ExtractedPacket::AuctionGetRequestsResponse(_)
        ));
    }

    #[test]
    fn buy_offer_request_exposes_cached_order_through_typed_default_flow() {
        let mut parser = parser();
        parser
            .extract_operation(
                "response",
                OperationCode::AuctionGetOffers,
                &market_order_params(14978117778),
                Some(0),
            )
            .unwrap();

        let mut params = BTreeMap::new();
        params.insert(1, json!(1));
        params.insert(2, json!(14978117778_i64));

        let extracted = parser
            .extract_operation("request", OperationCode::AuctionBuyOffer, &params, None)
            .unwrap();

        let ExtractedPacket::AuctionBuyOfferRequest(request) = extracted else {
            panic!("expected auction buy offer request");
        };

        assert_eq!(request.amount, Some(1));
        assert_eq!(request.order_id, Some(14978117778));
        assert_eq!(
            request.cached_order.as_ref().map(|order| order.id),
            request.order_id
        );
        assert_eq!(
            parser.unconfirmed_trade.as_ref().map(|trade| trade.id),
            Some(14978117778)
        );
        assert_eq!(
            parser
                .unconfirmed_trade
                .as_ref()
                .map(|trade| trade.operation.clone()),
            Some(OperationType::Buy)
        );
        assert_eq!(
            parser
                .unconfirmed_trade
                .as_ref()
                .and_then(|trade| trade.silver_amount),
            Some(5)
        );
    }

    #[test]
    fn sell_specific_item_and_trade_response_return_typed_packets() {
        let mut parser = parser();
        parser
            .extract_operation(
                "response",
                OperationCode::AuctionGetRequests,
                &market_order_params_with_type(14977174637, "request"),
                Some(0),
            )
            .unwrap();

        let mut params = BTreeMap::new();
        params.insert(1, json!(14977174637_i64));
        params.insert(4, json!(1));

        let extracted = parser
            .extract_operation(
                "request",
                OperationCode::AuctionSellSpecificItem,
                &params,
                None,
            )
            .unwrap();
        assert!(matches!(
            extracted,
            ExtractedPacket::AuctionSellSpecificItemRequest(_)
        ));

        let extracted = parser
            .extract_operation(
                "response",
                OperationCode::AuctionSellSpecificItem,
                &BTreeMap::new(),
                Some(0),
            )
            .unwrap();

        let ExtractedPacket::AuctionTradeResponse(response) = extracted else {
            panic!("expected auction trade response");
        };

        assert!(response.success);
        assert_eq!(
            response.confirmed_trade.as_ref().map(|trade| trade.id),
            Some(14977174637)
        );
        assert_eq!(
            response
                .confirmed_trade
                .as_ref()
                .map(|trade| trade.operation.clone()),
            Some(OperationType::Sell)
        );
        assert_eq!(
            response
                .confirmed_trade
                .as_ref()
                .map(|trade| trade.trade_type.clone()),
            Some(TradeType::Instant)
        );
        assert_eq!(
            response
                .confirmed_trade
                .as_ref()
                .and_then(|trade| trade.silver_amount),
            Some(5)
        );
        assert_eq!(
            response
                .confirmed_trade
                .as_ref()
                .and_then(|trade| trade.order.as_ref())
                .map(|order| order.id),
            Some(14977174637)
        );
    }

    #[test]
    fn trade_request_without_order_id_does_not_create_unconfirmed_trade() {
        let mut parser = parser();
        let mut params = BTreeMap::new();
        params.insert(1, json!(1));

        let extracted = parser
            .extract_operation("request", OperationCode::AuctionBuyOffer, &params, None)
            .unwrap();

        let ExtractedPacket::AuctionBuyOfferRequest(request) = extracted else {
            panic!("expected auction buy offer request");
        };

        assert_eq!(request.amount, Some(1));
        assert_eq!(request.order_id, None);
        assert_eq!(parser.unconfirmed_trade, None);

        let extracted = parser
            .extract_operation(
                "response",
                OperationCode::AuctionBuyOffer,
                &BTreeMap::new(),
                Some(0),
            )
            .unwrap();

        let ExtractedPacket::AuctionTradeResponse(response) = extracted else {
            panic!("expected auction trade response");
        };

        assert!(response.success);
        assert_eq!(response.confirmed_trade, None);
    }

    #[test]
    fn marketplace_notification_returns_typed_packet() {
        let mut parser = parser();
        let mut params = BTreeMap::new();
        params.insert(0, json!({"message": "sold"}));

        let extracted = parser
            .extract_event(EventCode::MarketPlaceNotification, &params)
            .unwrap();

        let ExtractedPacket::MarketPlaceNotification(notification) = extracted else {
            panic!("expected marketplace notification");
        };

        assert_eq!(notification.notification, json!({"message": "sold"}));
    }

    #[test]
    fn joined_chat_channel_returns_typed_packet() {
        let mut parser = parser();
        let mut params = BTreeMap::new();
        params.insert(0, json!(3));
        params.insert(1, json!("9001"));

        let extracted = parser
            .extract_event(EventCode::JoinedChatChannel, &params)
            .unwrap();

        let ExtractedPacket::JoinedChatChannel(response) = extracted else {
            panic!("expected joined chat channel");
        };

        assert_eq!(response.chat_index, 3);
        assert_eq!(response.channel_id, 9001);
    }

    #[test]
    fn left_chat_channel_returns_typed_packet() {
        let mut parser = parser();
        let mut params = BTreeMap::new();
        params.insert(0, json!("1856"));

        let extracted = parser
            .extract_event(EventCode::LeftChatChannel, &params)
            .unwrap();

        let ExtractedPacket::LeftChatChannel(response) = extracted else {
            panic!("expected left chat channel");
        };

        assert_eq!(response.channel_id, 1856);
    }

    #[test]
    fn joined_chat_channel_state_classifies_chat_messages() {
        let mut parser = parser();
        let mut join_params = BTreeMap::new();
        join_params.insert(0, json!(29));
        join_params.insert(1, json!(9001));

        parser
            .extract_event(EventCode::JoinedChatChannel, &join_params)
            .unwrap();

        let mut message_params = BTreeMap::new();
        message_params.insert(0, json!(9001));
        message_params.insert(1, json!("Player"));
        message_params.insert(2, json!("For the faction"));

        let extracted = parser
            .extract_event(EventCode::ChatMessage, &message_params)
            .unwrap();

        let ExtractedPacket::ChatMessage(message) = extracted else {
            panic!("expected chat message");
        };

        assert_eq!(message.channel_id, 9001);
        assert_eq!(message.channel_type, ChatChannel::Faction);
    }

    #[test]
    fn left_chat_channel_state_removes_chat_message_classification() {
        let mut parser = parser();
        let mut join_params = BTreeMap::new();
        join_params.insert(0, json!(29));
        join_params.insert(1, json!(9001));

        parser
            .extract_event(EventCode::JoinedChatChannel, &join_params)
            .unwrap();

        let mut left_params = BTreeMap::new();
        left_params.insert(0, json!(9001));

        parser
            .extract_event(EventCode::LeftChatChannel, &left_params)
            .unwrap();

        let mut message_params = BTreeMap::new();
        message_params.insert(0, json!(9001));
        message_params.insert(1, json!("Player"));
        message_params.insert(2, json!("No tracked channel"));

        let extracted = parser
            .extract_event(EventCode::ChatMessage, &message_params)
            .unwrap();

        let ExtractedPacket::ChatMessage(message) = extracted else {
            panic!("expected chat message");
        };

        assert_eq!(message.channel_id, 9001);
        assert_eq!(message.channel_type, ChatChannel::Say);
    }

    #[test]
    fn trade_silver_amount_accounts_for_scaled_distance_fee() {
        let mut parser = parser();
        parser
            .extract_operation(
                "response",
                OperationCode::AuctionGetOffers,
                &market_order_params_with_price(14978117778, "offer", 50_000, 20_000),
                Some(0),
            )
            .unwrap();

        let mut params = BTreeMap::new();
        params.insert(1, json!(3));
        params.insert(2, json!(14978117778_i64));

        parser
            .extract_operation("request", OperationCode::AuctionBuyOffer, &params, None)
            .unwrap();

        assert_eq!(
            parser
                .unconfirmed_trade
                .as_ref()
                .and_then(|trade| trade.silver_amount),
            Some(13)
        );
    }

    #[test]
    fn get_mail_infos_then_read_mail_returns_correlated_albion_mail() {
        let mut parser = parser();
        parser.player_state_mut().set_player_name("PlayerOne");

        let extracted = parser
            .extract_operation(
                "response",
                OperationCode::GetMailInfos,
                &mail_info_params(42),
                Some(0),
            )
            .unwrap();
        assert!(matches!(extracted, ExtractedPacket::GetMailInfos(_)));

        let extracted = parser
            .extract_operation(
                "response",
                OperationCode::ReadMail,
                &read_mail_params(42, "2|T4_HIDE|100000|50000"),
                Some(0),
            )
            .unwrap();

        let ExtractedPacket::AlbionMail(mail) = extracted else {
            panic!("expected correlated albion mail");
        };

        assert_eq!(mail.id, 42);
        assert_eq!(
            mail.location,
            AlbionLocation::with_names("2000", "Bridgewatch", "Bridgewatch")
        );
        assert_eq!(mail.player_name, "PlayerOne");
        assert_eq!(
            mail.info_type,
            MailInfoType::MarketPlaceSellOrderFinishedSummary
        );
        assert_eq!(mail.auction_type, AuctionType::Offer);
        assert_eq!(mail.received, 1_717_171_717);
        assert_eq!(mail.item_id, "T4_HIDE");
        assert_eq!(mail.item_name.as_deref(), Some("Adept's Hide"));
        assert_eq!(parser.albion_mails().get(&42), Some(&mail));
    }

    #[test]
    fn read_mail_before_get_mail_infos_is_cached_for_later_correlation() {
        let mut parser = parser();

        let extracted = parser.extract_operation(
            "response",
            OperationCode::ReadMail,
            &read_mail_params(42, "mail body"),
            Some(0),
        );
        assert!(extracted.is_none());
        assert!(parser.albion_mails().is_empty());

        let extracted = parser
            .extract_operation(
                "response",
                OperationCode::GetMailInfos,
                &mail_info_params(42),
                Some(0),
            )
            .unwrap();
        assert!(matches!(extracted, ExtractedPacket::GetMailInfos(_)));

        let mail = parser.albion_mails().get(&42).unwrap();
        assert_eq!(mail.id, 42);
        assert_eq!(
            mail.location,
            AlbionLocation::with_names("2000", "Bridgewatch", "Bridgewatch")
        );
        assert_eq!(mail.item_name, None);
    }

    #[test]
    fn mail_info_location_ids_are_normalized_before_correlation() {
        let mut parser = parser();
        let mut mail_info = mail_info_params(42);
        mail_info.insert(7, json!(["location@2000"]));

        parser
            .extract_operation("response", OperationCode::GetMailInfos, &mail_info, Some(0))
            .unwrap();

        let extracted = parser
            .extract_operation(
                "response",
                OperationCode::ReadMail,
                &read_mail_params(42, "mail body"),
                Some(0),
            )
            .unwrap();

        let ExtractedPacket::AlbionMail(mail) = extracted else {
            panic!("expected correlated albion mail");
        };

        assert_eq!(
            mail.location,
            AlbionLocation::with_names("2000", "Bridgewatch", "Bridgewatch")
        );
    }

    #[test]
    fn black_market_mail_location_maps_to_caerleon_index() {
        let mut parser = parser();
        let mut mail_info = mail_info_params(42);
        mail_info.insert(7, json!(["@BLACK_MARKET"]));

        parser
            .extract_operation("response", OperationCode::GetMailInfos, &mail_info, Some(0))
            .unwrap();

        let extracted = parser
            .extract_operation(
                "response",
                OperationCode::ReadMail,
                &read_mail_params(42, "mail body"),
                Some(0),
            )
            .unwrap();

        let ExtractedPacket::AlbionMail(mail) = extracted else {
            panic!("expected correlated albion mail");
        };

        assert_eq!(
            mail.location,
            AlbionLocation::with_names("3003", "Caerleon", "Caerleon")
        );
    }

    #[test]
    fn incomplete_get_mail_infos_rows_are_skipped_without_panic() {
        let mut parser = parser();
        let mut params = BTreeMap::new();
        params.insert(3, json!([42, 43]));
        params.insert(7, json!(["2000"]));
        params.insert(11, json!(["MARKETPLACE_SELLORDER_FINISHED_SUMMARY"]));
        params.insert(12, json!([]));

        let extracted = parser
            .extract_operation("response", OperationCode::GetMailInfos, &params, Some(0))
            .unwrap();

        let ExtractedPacket::GetMailInfos(response) = extracted else {
            panic!("expected get mail infos");
        };

        assert_eq!(response.mail_ids, vec![42, 43]);
        assert!(parser.albion_mails().is_empty());
    }

    #[test]
    fn extracted_json_is_explicit_escape_hatch() {
        let packet = DecodedPacket::Operation(DecodedOperation {
            file: "test".to_string(),
            packet_number: 1,
            direction: "client_to_server".to_string(),
            source: "client:1".to_string(),
            destination: "server:5056".to_string(),
            message_type: "operation_request".to_string(),
            code: OperationCode::AuctionBuyOffer,
            name: OperationCode::AuctionBuyOffer.name().to_string(),
            return_code: None,
            debug_message: String::new(),
            parameters: BTreeMap::new(),
            extracted: Some(ExtractedPacket::AuctionBuyOfferRequest(AuctionBuyOffer {
                amount: Some(1),
                cached_order: None,
                order_id: Some(14978117778),
            })),
        });

        assert_eq!(
            packet.extracted_json(),
            Some(json!({
                "amount": 1,
                "cached_order": null,
                "order_id": 14978117778_i64
            }))
        );
    }

    #[test]
    fn event_extracted_json_is_explicit_escape_hatch() {
        let packet = DecodedPacket::Event(DecodedEvent {
            file: "test".to_string(),
            packet_number: 1,
            direction: "server_to_client".to_string(),
            source: "server:5056".to_string(),
            destination: "client:1".to_string(),
            message_type: "event".to_string(),
            code: EventCode::MarketPlaceNotification,
            name: EventCode::MarketPlaceNotification.name().to_string(),
            return_code: None,
            debug_message: String::new(),
            parameters: BTreeMap::new(),
            extracted: Some(ExtractedPacket::MarketPlaceNotification(
                MarketPlaceNotification {
                    notification: json!({"message": "sold"}),
                },
            )),
        });

        assert_eq!(
            packet.extracted_json(),
            Some(json!({
                "notification": {
                    "message": "sold"
                }
            }))
        );
    }

    #[test]
    fn unknown_extracted_json_is_none() {
        let packet = DecodedPacket::Unknown(DecodedUnknown {
            file: "test".to_string(),
            packet_number: 1,
            direction: "server_to_client".to_string(),
            source: "server:5056".to_string(),
            destination: "client:1".to_string(),
            message_type: "event".to_string(),
            kind: "event".to_string(),
            code_parameter: 252,
            raw_code: Some(32_767),
            reason: "unknown_event_code".to_string(),
            return_code: None,
            debug_message: String::new(),
            parameters: BTreeMap::new(),
        });

        assert_eq!(packet.extracted_json(), None);
    }

    fn mail_info_params(mail_id: i64) -> BTreeMap<u8, Value> {
        let mut params = BTreeMap::new();
        params.insert(3, json!([mail_id]));
        params.insert(7, json!(["2000"]));
        params.insert(11, json!(["MARKETPLACE_SELLORDER_FINISHED_SUMMARY"]));
        params.insert(12, json!([1_717_171_717_i64]));
        params
    }

    fn read_mail_params(mail_id: i64, mail_string: &str) -> BTreeMap<u8, Value> {
        let mut params = BTreeMap::new();
        params.insert(0, json!(mail_id));
        params.insert(1, json!(mail_string));
        params
    }

    fn market_order_params(order_id: i64) -> BTreeMap<u8, Value> {
        market_order_params_with_type(order_id, "offer")
    }

    fn market_order_params_with_type(order_id: i64, auction_type: &str) -> BTreeMap<u8, Value> {
        market_order_params_with_price(order_id, auction_type, 50_000, 0)
    }

    fn market_order_params_with_price(
        order_id: i64,
        auction_type: &str,
        unit_price_silver: i64,
        distance_fee: i64,
    ) -> BTreeMap<u8, Value> {
        let mut params = BTreeMap::new();
        params.insert(
            0,
            json!([
                {
                    "Amount": 2,
                    "AuctionType": auction_type,
                    "BuyerCharacterId": null,
                    "BuyerName": null,
                    "DistanceFee": distance_fee,
                    "EnchantmentLevel": 0,
                    "Expires": "2026-06-22T03:34:16.096699",
                    "HasBuyerFetched": false,
                    "HasSellerFetched": false,
                    "Id": order_id,
                    "IsFinished": false,
                    "ItemGroupTypeId": "T1_HIDE",
                    "ItemTypeId": "T1_HIDE",
                    "LocationId": null,
                    "QualityLevel": 1,
                    "ReferenceId": "7ae09894-4883-479b-932e-ff7914c82855",
                    "SellerCharacterId": "07b8fbc0-c512-4054-bc53-12312af94df3",
                    "SellerName": "CoelhoMalvado",
                    "Tier": 1,
                    "TotalPriceSilver": unit_price_silver * 2,
                    "UnitPriceSilver": unit_price_silver
                }
            ]),
        );
        params
    }
}
