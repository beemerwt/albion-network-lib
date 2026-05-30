use crate::albion::{
    PlayerState, WorldMap, AlbionMail,
};
use crate::protocol::Protocol18Deserializer;

use std::{
    sync::Arc,
    collections::HashMap,
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
        debug: bool,
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
                    destination
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
        let operation_code = match parse_operation_code(&parameters, self.debug) {
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
                    timestamp: Utc::now().timestamp_millis(),
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
                    timestamp: Utc::now().timestamp_millis(),
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
