use super::provider::{DiProvider, Provider};
use crate::models::database;
use crate::schema::accounts;
use anyhow::Result;
use core::cmp::Ordering;
use diesel::{BelongingToDsl, ExpressionMethods, QueryDsl, RunQueryDsl};
use glib::subclass::{self, prelude::*};
use glib::{Cast, ObjectExt, StaticType, ToValue};
use std::cell::{Cell, RefCell};

#[derive(Insertable)]
#[table_name = "accounts"]
struct NewAccount {
    pub name: String,
    pub token_id: String,
    pub provider_id: i32,
}

#[derive(Identifiable, Queryable, Associations, Hash, PartialEq, Eq, Debug, Clone)]
#[belongs_to(DiProvider, foreign_key = "provider_id")]
#[table_name = "accounts"]
pub struct DiAccount {
    pub id: i32,
    pub name: String,
    pub token_id: String,
    pub provider_id: i32,
}

pub struct AccountPriv {
    pub id: Cell<i32>,
    pub name: RefCell<String>,
    pub token_id: RefCell<String>,
    pub provider_id: Cell<i32>,
}

static PROPERTIES: [subclass::Property; 4] = [
    subclass::Property("id", |name| {
        glib::ParamSpec::int(name, "id", "Id", 0, 1000, 0, glib::ParamFlags::READWRITE)
    }),
    subclass::Property("name", |name| {
        glib::ParamSpec::string(name, "name", "Name", None, glib::ParamFlags::READWRITE)
    }),
    subclass::Property("token-id", |name| {
        glib::ParamSpec::string(
            name,
            "token-id",
            "token id",
            None,
            glib::ParamFlags::READWRITE,
        )
    }),
    subclass::Property("provider-id", |name| {
        glib::ParamSpec::int(
            name,
            "provider-id",
            "Provider Id",
            0,
            1000,
            0,
            glib::ParamFlags::READWRITE,
        )
    }),
];

impl ObjectSubclass for AccountPriv {
    const NAME: &'static str = "Account";
    type Type = super::Account;
    type ParentType = glib::Object;
    type Instance = subclass::simple::InstanceStruct<Self>;
    type Class = subclass::simple::ClassStruct<Self>;

    glib_object_subclass!();

    fn class_init(klass: &mut Self::Class) {
        klass.install_properties(&PROPERTIES);
    }

    fn new() -> Self {
        Self {
            id: Cell::new(0),
            name: RefCell::new("".to_string()),
            token_id: RefCell::new("".to_string()),
            provider_id: Cell::new(0),
        }
    }
}

impl ObjectImpl for AccountPriv {
    fn set_property(&self, _obj: &Self::Type, id: usize, value: &glib::Value) {
        let prop = &PROPERTIES[id];

        match *prop {
            subclass::Property("id", ..) => {
                let id = value
                    .get()
                    .expect("type conformity checked by `Object::set_property`")
                    .unwrap();
                self.id.replace(id);
            }
            subclass::Property("name", ..) => {
                let name = value
                    .get()
                    .expect("type conformity checked by `Object::set_property`")
                    .unwrap();
                self.name.replace(name);
            }
            subclass::Property("token-id", ..) => {
                let token_id = value
                    .get()
                    .expect("type conformity checked by `Object::set_property`")
                    .unwrap();
                self.token_id.replace(token_id);
            }
            subclass::Property("provider-id", ..) => {
                let provider_id = value
                    .get()
                    .expect("type conformity checked by `Object::set_property`")
                    .unwrap();
                self.provider_id.replace(provider_id);
            }
            _ => unimplemented!(),
        }
    }

    fn get_property(&self, _obj: &Self::Type, id: usize) -> Result<glib::Value, ()> {
        let prop = &PROPERTIES[id];

        match *prop {
            subclass::Property("id", ..) => Ok(self.id.get().to_value()),
            subclass::Property("name", ..) => Ok(self.name.borrow().to_value()),
            subclass::Property("token-id", ..) => Ok(self.token_id.borrow().to_value()),
            subclass::Property("provider-id", ..) => Ok(self.provider_id.get().to_value()),
            _ => unimplemented!(),
        }
    }
}

glib_wrapper! {
    pub struct Account(ObjectSubclass<AccountPriv>);
}

impl Account {
    pub fn create(name: &str, token_id: &str, provider_id: i32) -> Result<Account> {
        let db = database::connection();
        let conn = db.get()?;

        diesel::insert_into(accounts::table)
            .values(NewAccount {
                name: name.to_string(),
                token_id: token_id.to_string(),
                provider_id,
            })
            .execute(&conn)?;

        accounts::table
            .order(accounts::columns::id.desc())
            .first::<DiAccount>(&conn)
            .map_err(From::from)
            .map(From::from)
    }

    pub fn load(p: &Provider) -> Result<Vec<Self>> {
        let db = database::connection();
        let conn = db.get()?;

        let dip: DiProvider = p.into();
        let results = DiAccount::belonging_to(&dip)
            .load::<DiAccount>(&conn)?
            .into_iter()
            .map(From::from)
            .collect::<Vec<Account>>();
        Ok(results)
    }

    pub fn compare(obj1: &glib::Object, obj2: &glib::Object) -> Ordering {
        let account1 = obj1.downcast_ref::<Account>().unwrap();
        let account2 = obj2.downcast_ref::<Account>().unwrap();

        account1.name().cmp(&account2.name())
    }

    pub fn new(id: i32, name: &str, token_id: &str, provider_id: i32) -> Account {
        glib::Object::new(
            Account::static_type(),
            &[
                ("id", &id),
                ("name", &name),
                ("token-id", &token_id),
                ("provider-id", &provider_id),
            ],
        )
        .expect("Failed to create account")
        .downcast()
        .expect("Created account is of wrong type")
    }

    pub fn id(&self) -> i32 {
        let priv_ = AccountPriv::from_instance(self);
        priv_.id.get()
    }

    pub fn name(&self) -> String {
        let priv_ = AccountPriv::from_instance(self);
        priv_.name.borrow().clone()
    }

    pub fn set_name(&self, name: &str) {
        self.set_property("name", &name)
            .expect("Failed to set `name` property");
    }

    pub fn delete(&self) -> Result<()> {
        Ok(())
    }
}

impl From<DiAccount> for Account {
    fn from(account: DiAccount) -> Self {
        Self::new(
            account.id,
            &account.name,
            &account.token_id,
            account.provider_id,
        )
    }
}
