use crate::albion::{
    AlbionMail, ItemNameResolver, MailInfoMetadata, WorldMap,
    payloads::{GetMailInfos, ReadMail},
};
use std::collections::HashMap;

#[derive(Clone, Debug, Default)]
pub struct MailState {
    mail_infos_by_id: HashMap<i64, MailInfoMetadata>,
    read_mails_by_id: HashMap<i64, ReadMail>,
    albion_mails_by_id: HashMap<i64, AlbionMail>,
}

impl MailState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn albion_mails(&self) -> &HashMap<i64, AlbionMail> {
        &self.albion_mails_by_id
    }

    pub fn get_albion_mail(&self, mail_id: i64) -> Option<&AlbionMail> {
        self.albion_mails_by_id.get(&mail_id)
    }

    pub fn cache_mail_infos(
        &mut self,
        response: &GetMailInfos,
        world_map: &WorldMap,
        item_names: &ItemNameResolver,
        player_name: &str,
    ) {
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
                let mail =
                    build_albion_mail(&metadata, &read_mail, world_map, item_names, player_name);

                self.albion_mails_by_id.insert(mail.id, mail);
            }
        }
    }

    pub fn cache_read_mail(
        &mut self,
        response: ReadMail,
        world_map: &WorldMap,
        item_names: &ItemNameResolver,
        player_name: &str,
    ) -> Option<AlbionMail> {
        self.read_mails_by_id
            .insert(response.mail_id, response.clone());

        let metadata = self.mail_infos_by_id.get(&response.mail_id)?.clone();
        let mail = build_albion_mail(&metadata, &response, world_map, item_names, player_name);
        self.albion_mails_by_id.insert(mail.id, mail.clone());

        Some(mail)
    }
}

fn build_albion_mail(
    metadata: &MailInfoMetadata,
    read_mail: &ReadMail,
    world_map: &WorldMap,
    item_names: &ItemNameResolver,
    player_name: &str,
) -> AlbionMail {
    let mut mail = AlbionMail::from_correlated(
        metadata.mail_id,
        world_map.resolve_location(&metadata.location_id),
        player_name.to_string(),
        metadata.info_type,
        metadata.received,
        &read_mail.mail_string,
    );

    mail.item_name = item_names.resolve_owned(&mail.item_id);
    mail
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
