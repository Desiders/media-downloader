use std::convert::Infallible;

use sea_orm::{ActiveValue::Set, ConnectionTrait, EntityTrait, sea_query::OnConflict};

use crate::{adapters::database::models::chats, entities::Chat, errors::database::ErrorKind};

pub struct Dao<'a, Conn> {
    conn: &'a Conn,
}

impl<'a, Conn> Dao<'a, Conn> {
    pub const fn new(conn: &'a Conn) -> Self
    where
        Conn: ConnectionTrait,
    {
        Self { conn }
    }
}

impl<Conn> Dao<'_, Conn>
where
    Conn: ConnectionTrait,
{
    pub async fn insert_or_update(
        &self,
        Chat {
            tg_id,
            username,
            created_at,
            updated_at,
        }: Chat,
    ) -> Result<(), ErrorKind<Infallible>> {
        use chats::{
            ActiveModel,
            Column::{TgId, UpdatedAt, Username},
            Entity,
        };

        let model = ActiveModel {
            tg_id: Set(tg_id),
            username: Set(username.map(Into::into)),
            created_at: Set(created_at),
            updated_at: Set(updated_at),
        };

        Entity::insert(model)
            .on_conflict(OnConflict::column(TgId).update_columns([Username, UpdatedAt]).to_owned())
            .exec_without_returning(self.conn)
            .await
            .map(|_| ())
            .map_err(Into::into)
    }
}
