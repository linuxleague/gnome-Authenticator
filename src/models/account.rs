use crate::schema::accounts;

use crate::models::database;

use diesel::RunQueryDsl;
pub use failure::Error;

use diesel::prelude::*;

#[derive(Queryable, PartialEq, Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    pub id: i32,
    pub username: String,
    pub token_id: String,
    pub provider: i32,
}

#[derive(Insertable)]
#[table_name = "accounts"]
pub struct NewAccount {
    pub username: String,
    pub token_id: String,
    pub provider: i32,
}

impl database::Insert<Account> for NewAccount {
    type Error = database::Error;
    fn insert(&self) -> Result<Account, database::Error> {
        let db = database::connection();
        let conn = db.get()?;

        diesel::insert_into(accounts::table).values(self).execute(&conn)?;

        accounts::table.order(accounts::columns::id.desc()).first::<Account>(&conn).map_err(From::from)
    }
}
