use crate::schema::providers;

use crate::models::database;

use diesel::RunQueryDsl;
pub use failure::Error;

use diesel::prelude::*;

#[derive(Queryable, Hash, PartialEq, Eq, Debug, Clone, Serialize, Deserialize)]
pub struct Provider {
    pub id: i32,
    pub name: String,
    pub website: String,
    pub help_url: String,
    pub image_uri: String,
}

#[derive(Insertable)]
#[table_name = "providers"]
pub struct NewProvider {
    pub name: String,
    pub website: String,
    pub help_url: String,
    pub image_uri: String,
}

impl database::Insert<Provider> for NewProvider {
    type Error = database::Error;
    fn insert(&self) -> Result<Provider, database::Error> {
        let db = database::connection();
        let conn = db.get()?;

        diesel::insert_into(providers::table)
            .values(self)
            .execute(&conn)?;

        providers::table
            .order(providers::columns::id.desc())
            .first::<Provider>(&conn)
            .map_err(From::from)
    }
}
