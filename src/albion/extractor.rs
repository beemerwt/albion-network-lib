use std::sync::Arc;

use crate::{
    albion::{
        AlbionLocation, CachedOrder, EventCode, ExtractedPacket, ItemNameResolver, OperationCode,
        PlayerState, WorldMap,
        chat_state::ChatState,
        mail_state::MailState,
        market_state::MarketState,
        payloads::{
            AuctionGetOffers, AuctionGetOffersResult, AuctionGetRequests, AuctionGetRequestsResult,
            ChatMessage, GetMailInfos, JoinResponse, JoinedChatChannel, LeftChatChannel,
            MarketPlaceNotification, ReadMail,
        },
    },
    packet::{OperationPacketKind, RawParameters},
    util::value_i64,
};
use serde_json::Value;

pub struct AlbionExtractor {
    pub world_map: Arc<WorldMap>,
    pub item_names: ItemNameResolver,
    pub player_state: PlayerState,
    pub mail_state: MailState,
    pub market_state: MarketState,
    pub chat_state: ChatState,
}

impl AlbionExtractor {
    pub fn new(world_map: Arc<WorldMap>, item_names: ItemNameResolver) -> Self {
        Self {
            world_map,
            item_names,
            player_state: PlayerState::new(),
            market_state: MarketState::new(),
            mail_state: MailState::new(),
            chat_state: ChatState::new(),
        }
    }

    pub fn with_defaults() -> Self {
        Self::new(
            Arc::new(WorldMap::from_embedded().unwrap_or_else(|_| WorldMap::empty())),
            ItemNameResolver::empty(),
        )
    }

    pub fn player_state(&self) -> &PlayerState {
        &self.player_state
    }

    pub fn market_order_count(&self) -> usize {
        self.market_state.order_count()
    }

    pub fn albion_mails(&self) -> &std::collections::HashMap<i64, crate::albion::AlbionMail> {
        self.mail_state.albion_mails()
    }

    pub(crate) fn player_state_mut(&mut self) -> &mut PlayerState {
        &mut self.player_state
    }

    pub fn mark_encrypted_data_seen(&mut self) {
        self.player_state.set_has_encrypted_data(true);
    }

    pub fn extract_operation(
        &mut self,
        packet_kind: OperationPacketKind,
        operation_code: OperationCode,
        parameters: &RawParameters,
        return_code: Option<i16>,
    ) -> Option<ExtractedPacket> {
        let fallback_location_index = self.player_state.location_index();
        match (operation_code, packet_kind) {
            (OperationCode::AuctionGetOffers, OperationPacketKind::Request) => {
                let orders = self.extract_market_orders(parameters, fallback_location_index);
                return Some(ExtractedPacket::AuctionGetOffersRequest(AuctionGetOffers {
                    market_order_count: orders.len(),
                    market_orders: orders,
                }));
            }
            (OperationCode::AuctionGetRequests, OperationPacketKind::Request) => {
                let orders = self.extract_market_orders(parameters, fallback_location_index);
                return Some(ExtractedPacket::AuctionGetRequestsRequest(
                    AuctionGetRequests {
                        market_order_count: orders.len(),
                        market_orders: orders,
                    },
                ));
            }
            (OperationCode::AuctionGetOffers, OperationPacketKind::Response) => {
                let orders = self.extract_market_orders(parameters, fallback_location_index);
                self.market_state.cache_orders_from_slice(&orders);
                return Some(ExtractedPacket::AuctionGetOffersResponse(
                    AuctionGetOffersResult {
                        market_order_count: orders.len(),
                        market_orders: orders,
                    },
                ));
            }
            (OperationCode::AuctionGetRequests, OperationPacketKind::Response) => {
                let orders = self.extract_market_orders(parameters, fallback_location_index);
                self.market_state.cache_orders_from_slice(&orders);
                return Some(ExtractedPacket::AuctionGetRequestsResponse(
                    AuctionGetRequestsResult {
                        market_order_count: orders.len(),
                        market_orders: orders,
                    },
                ));
            }
            (OperationCode::AuctionBuyOffer, OperationPacketKind::Request) => {
                let amount = parameters.get(1).and_then(value_i64);
                let order_id = parameters.get(2).and_then(value_i64);
                let request = self.market_state.begin_buy_order_request(order_id, amount);
                return Some(ExtractedPacket::AuctionBuyOfferRequest(request));
            }
            (OperationCode::AuctionSellSpecificItem, OperationPacketKind::Request) => {
                let amount = parameters.get(4).and_then(value_i64);
                let order_id = parameters.get(1).and_then(value_i64);
                let request = self
                    .market_state
                    .begin_sell_specific_item_request(order_id, amount);
                return Some(ExtractedPacket::AuctionSellSpecificItemRequest(request));
            }
            (
                OperationCode::AuctionBuyOffer | OperationCode::AuctionSellSpecificItem,
                OperationPacketKind::Response,
            ) => {
                let response = self.market_state.finish_instant_trade_response(return_code);
                return Some(ExtractedPacket::AuctionTradeResponse(response));
            }
            (OperationCode::Join, OperationPacketKind::Response) => {
                return Some(self.handle_join_response(parameters));
            }
            (OperationCode::GetMailInfos, OperationPacketKind::Response) => {
                return Some(self.handle_get_mail_infos_response(parameters));
            }
            (OperationCode::ReadMail, OperationPacketKind::Response) => {
                return self.handle_read_mail_response(parameters);
            }
            _ => {}
        }

        None
    }

    fn handle_join_response(&mut self, parameters: &RawParameters) -> ExtractedPacket {
        let response = JoinResponse::from_params(parameters);
        self.player_state
            .set_user_object_id(response.user_object_id);
        if let Some(player_name) = response.player_name.as_deref() {
            self.player_state.set_player_name(player_name);
        }

        let location = self.world_map.resolve_location(&response.player_location);
        self.player_state.set_location(location);

        ExtractedPacket::JoinResponse(response)
    }

    fn handle_get_mail_infos_response(&mut self, parameters: &RawParameters) -> ExtractedPacket {
        let response = GetMailInfos::from_params(parameters);

        self.mail_state.cache_mail_infos(
            &response,
            &self.world_map,
            &self.item_names,
            self.player_state.player_name(),
        );

        ExtractedPacket::GetMailInfos(response)
    }

    fn handle_read_mail_response(&mut self, parameters: &RawParameters) -> Option<ExtractedPacket> {
        let response = ReadMail::from_params(parameters);
        self.mail_state
            .cache_read_mail(
                response,
                &self.world_map,
                &self.item_names,
                self.player_state.player_name(),
            )
            .map(ExtractedPacket::AlbionMail)
    }

    pub fn extract_market_orders(
        &self,
        params: &RawParameters,
        fallback_location_index: Option<&str>,
    ) -> Vec<CachedOrder> {
        let Some(raw_orders) = params.get(0) else {
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
                order.item_name = self.item_names.resolve_owned(&order.item_id);
                order
            })
            .collect()
    }

    pub fn extract_event(
        &mut self,
        event_code: EventCode,
        parameters: &RawParameters,
    ) -> Option<ExtractedPacket> {
        match event_code {
            EventCode::MarketPlaceNotification => {
                Some(self.extract_marketplace_notification(parameters))
            }

            EventCode::ChatMessage => Some(self.extract_chat_message(parameters)),

            EventCode::ChatSay => Some(ExtractedPacket::ChatMessage(ChatMessage::from_say_params(
                parameters,
            ))),

            EventCode::JoinedChatChannel => Some(self.extract_joined_chat_channel(parameters)),

            EventCode::LeftChatChannel => Some(self.extract_left_chat_channel(parameters)),
            _ => None,
        }
    }

    fn extract_marketplace_notification(&self, parameters: &RawParameters) -> ExtractedPacket {
        ExtractedPacket::MarketPlaceNotification(MarketPlaceNotification {
            notification: parameters.get(0).cloned().unwrap_or(Value::Null),
        })
    }

    fn extract_chat_message(&self, parameters: &RawParameters) -> ExtractedPacket {
        let channel_type = parameters
            .get(0)
            .and_then(value_i64)
            .and_then(|channel_id| self.chat_state.channel_type(channel_id));

        ExtractedPacket::ChatMessage(ChatMessage::from_params_with_channel_type(
            parameters,
            channel_type,
        ))
    }

    fn extract_joined_chat_channel(&mut self, parameters: &RawParameters) -> ExtractedPacket {
        let response = JoinedChatChannel::from_params(parameters);

        self.chat_state
            .join_channel(response.channel_id, i64::from(response.chat_index));

        ExtractedPacket::JoinedChatChannel(response)
    }

    fn extract_left_chat_channel(&mut self, parameters: &RawParameters) -> ExtractedPacket {
        let response = LeftChatChannel::from_params(parameters);

        self.chat_state.leave_channel(response.channel_id);

        ExtractedPacket::LeftChatChannel(response)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

use super::*;
    use crate::{albion::WorldMap, packet::OperationPacketKind};
    use serde_json::json;

    fn test_extractor() -> AlbionExtractor {
        let world_map = Arc::new(WorldMap::from_embedded().unwrap());

        let item_names = ItemNameResolver::new(HashMap::from([
            ("T1_HIDE".to_string(), "Scraps of Hide".to_string()),
            ("T4_HIDE".to_string(), "Adept's Hide".to_string()),
        ]));

        AlbionExtractor::new(world_map, item_names)
    }

    #[test]
    fn join_response_updates_player_identity_and_resolved_location() {
        let mut extractor = test_extractor();

        let mut params = RawParameters::empty();
        params.insert(0, json!(123));
        params.insert(2, json!("PlayerOne"));
        params.insert(8, json!("Bridgewatch"));

        let extracted = extractor
            .extract_operation(
                OperationPacketKind::Response,
                OperationCode::Join,
                &params,
                Some(0),
            )
            .unwrap();

        assert!(matches!(extracted, ExtractedPacket::JoinResponse(_)));

        let state = extractor.player_state();

        assert_eq!(state.user_object_id(), Some(123));
        assert_eq!(state.player_name(), "PlayerOne");
        assert_eq!(state.location().friendly_name(), "Bridgewatch");
        assert_eq!(state.location_id(), Some(2000));
    }
}
