use super::{
    provider::{DiProvider, Provider},
    OTPMethod, OTPUri,
};
use crate::models::otp;
use crate::widgets::QRCodeData;
use crate::{helpers::Keyring, models::database, schema::accounts};
use anyhow::Result;
use core::cmp::Ordering;
use diesel::{BelongingToDsl, ExpressionMethods, QueryDsl, RunQueryDsl};
use glib::{clone, Cast, ObjectExt, StaticType, ToValue};
use gtk::glib;
use gtk::subclass::prelude::*;
use once_cell::sync::OnceCell;
use std::{
    cell::{Cell, RefCell},
    time::{SystemTime, UNIX_EPOCH},
};
use unicase::UniCase;

#[derive(Insertable)]
#[table_name = "accounts"]
struct NewAccount {
    pub name: String,
    pub token_id: String,
    pub provider_id: i32,
    pub counter: i32,
}

#[derive(Identifiable, Queryable, Associations, Hash, PartialEq, Eq, Debug, Clone)]
#[belongs_to(DiProvider, foreign_key = "provider_id")]
#[table_name = "accounts"]
pub struct DiAccount {
    pub id: i32,
    pub name: String,
    pub counter: i32,
    pub token_id: String,
    pub provider_id: i32,
}

#[doc(hidden)]
mod imp {
    use super::*;
    use glib::subclass;

    static PROPERTIES: [subclass::Property; 6] = [
        subclass::Property("id", |name| {
            glib::ParamSpec::int(
                name,
                "id",
                "Id",
                0,
                i32::MAX,
                0,
                glib::ParamFlags::READWRITE,
            )
        }),
        subclass::Property("counter", |name| {
            glib::ParamSpec::int(
                name,
                "counter",
                "Counter",
                0,
                i32::MAX,
                0,
                glib::ParamFlags::READWRITE,
            )
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
        subclass::Property("otp", |name| {
            glib::ParamSpec::string(
                name,
                "otp",
                "The One Time Password",
                None,
                glib::ParamFlags::READWRITE,
            )
        }),
        subclass::Property("provider", |name| {
            glib::ParamSpec::object(
                name,
                "provider",
                "The account provider",
                Provider::static_type(),
                glib::ParamFlags::READWRITE,
            )
        }),
    ];
    pub struct Account {
        pub id: Cell<i32>,
        pub otp: RefCell<String>,
        pub name: RefCell<String>,
        pub counter: Cell<i32>,
        pub token: OnceCell<String>,
        pub token_id: RefCell<String>,
        pub provider: RefCell<Option<Provider>>,
    }

    impl ObjectSubclass for Account {
        const NAME: &'static str = "Account";
        type Type = super::Account;
        type ParentType = glib::Object;
        type Instance = subclass::simple::InstanceStruct<Self>;
        type Class = subclass::simple::ClassStruct<Self>;

        glib::object_subclass!();

        fn class_init(klass: &mut Self::Class) {
            klass.install_properties(&PROPERTIES);
        }

        fn new() -> Self {
            Self {
                id: Cell::new(0),
                counter: Cell::new(1),
                name: RefCell::new("".to_string()),
                otp: RefCell::new("".to_string()),
                token_id: RefCell::new("".to_string()),
                provider: RefCell::new(None),
                token: OnceCell::new(),
            }
        }
    }

    impl ObjectImpl for Account {
        fn set_property(&self, _obj: &Self::Type, id: usize, value: &glib::Value) {
            let prop = &PROPERTIES[id];

            match *prop {
                subclass::Property("id", ..) => {
                    let id = value.get().unwrap().unwrap();
                    self.id.replace(id);
                }
                subclass::Property("name", ..) => {
                    let name = value.get().unwrap().unwrap();
                    self.name.replace(name);
                }
                subclass::Property("counter", ..) => {
                    let counter = value.get().unwrap().unwrap();
                    self.counter.replace(counter);
                }
                subclass::Property("otp", ..) => {
                    let otp = value.get().unwrap().unwrap();
                    self.otp.replace(otp);
                }
                subclass::Property("token-id", ..) => {
                    let token_id = value.get().unwrap().unwrap();
                    self.token_id.replace(token_id);
                }
                subclass::Property("provider", ..) => {
                    let provider = value.get().unwrap();
                    self.provider.replace(provider);
                }
                _ => unimplemented!(),
            }
        }

        fn get_property(&self, _obj: &Self::Type, id: usize) -> glib::Value {
            let prop = &PROPERTIES[id];

            match *prop {
                subclass::Property("id", ..) => self.id.get().to_value(),
                subclass::Property("name", ..) => self.name.borrow().to_value(),
                subclass::Property("counter", ..) => self.counter.get().to_value(),
                subclass::Property("otp", ..) => self.otp.borrow().to_value(),
                subclass::Property("token-id", ..) => self.token_id.borrow().to_value(),
                subclass::Property("provider", ..) => self.provider.borrow().to_value(),
                _ => unimplemented!(),
            }
        }
    }
}
glib::wrapper! {
    pub struct Account(ObjectSubclass<imp::Account>);
}

impl Account {
    pub fn create(name: &str, token: &str, provider: &Provider) -> Result<Account> {
        let db = database::connection();
        let conn = db.get()?;

        let token_id = Keyring::store(&format!("{} - {}", provider.name(), name), &token)
            .expect("Failed to save token");

        diesel::insert_into(accounts::table)
            .values(NewAccount {
                name: name.to_string(),
                token_id,
                provider_id: provider.id(),
                counter: provider.default_counter(),
            })
            .execute(&conn)?;

        accounts::table
            .order(accounts::columns::id.desc())
            .first::<DiAccount>(&conn)
            .map_err(From::from)
            .map(|account| {
                Self::new(
                    account.id,
                    &account.name,
                    &account.token_id,
                    account.counter,
                    provider.clone(),
                )
            })
    }

    pub fn load(p: &Provider) -> Result<impl Iterator<Item = Self>> {
        let db = database::connection();
        let conn = db.get()?;

        let dip: DiProvider = p.into();
        let results = DiAccount::belonging_to(&dip)
            .load::<DiAccount>(&conn)?
            .into_iter()
            .map(clone!(@weak p => move |account| {
                Self::new(
                    account.id,
                    &account.name,
                    &account.token_id,
                    account.counter,
                    p,
                )
            }));

        Ok(results)
    }

    pub fn compare(obj1: &glib::Object, obj2: &glib::Object) -> Ordering {
        let account1 = obj1.downcast_ref::<Account>().unwrap();
        let account2 = obj2.downcast_ref::<Account>().unwrap();

        UniCase::new(account1.name()).cmp(&UniCase::new(account2.name()))
    }

    pub fn new(id: i32, name: &str, token_id: &str, counter: i32, provider: Provider) -> Account {
        let account = glib::Object::new(&[
            ("id", &id),
            ("name", &name),
            ("token-id", &token_id),
            ("provider", &provider),
            ("counter", &counter),
        ])
        .expect("Failed to create account");

        let token = Keyring::token(token_id).unwrap().unwrap();
        let self_ = imp::Account::from_instance(&account);
        self_.token.set(token).unwrap();

        account
    }

    pub fn generate_otp(&self) {
        let provider = self.provider();

        let counter = match provider.method() {
            OTPMethod::TOTP => {
                let timestamp = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs();
                timestamp / (provider.period() as u64)
            }
            OTPMethod::HOTP => {
                let old_counter = self.counter();
                self.increment_counter();
                old_counter as u64
            }
            OTPMethod::Steam => 1,
        };

        let label = match otp::generate_hotp(
            &self.token(),
            counter,
            provider.algorithm().into(),
            provider.digits() as u32,
        ) {
            Ok(otp) => otp::format(otp, provider.digits() as usize),
            Err(err) => {
                debug!("Could not generate HOTP {:?}", err);
                "Error".to_string()
            }
        };
        self.set_property("otp", &label).unwrap();
    }

    fn increment_counter(&self) -> Result<()> {
        // For security reasons, never re-use the same counter for HOTP
        let self_ = imp::Account::from_instance(self);
        let new_value = self.counter() + 1;
        self_.counter.set(new_value);

        let db = database::connection();
        let conn = db.get()?;

        let target = accounts::table.filter(accounts::columns::id.eq(self.id()));
        diesel::update(target)
            .set(accounts::columns::counter.eq(new_value))
            .execute(&conn)?;
        Ok(())
    }

    pub fn copy_otp(&self) {
        let display = gtk::gdk::Display::get_default().unwrap();
        let clipboard = display.get_clipboard();
        let self_ = imp::Account::from_instance(self);
        clipboard.set_text(&self_.otp.borrow());

        // Indirectly increment the counter once the token was copied
        if self.provider().method() == OTPMethod::HOTP {
            self.generate_otp();
        }
    }

    pub fn id(&self) -> i32 {
        let self_ = imp::Account::from_instance(self);
        self_.id.get()
    }

    pub fn provider(&self) -> Provider {
        let provider = self.get_property("provider").unwrap();
        provider.get::<Provider>().unwrap().unwrap()
    }

    pub fn counter(&self) -> i32 {
        let self_ = imp::Account::from_instance(self);
        self_.counter.get()
    }

    pub fn name(&self) -> String {
        let self_ = imp::Account::from_instance(self);
        self_.name.borrow().clone()
    }

    pub fn token(&self) -> String {
        let self_ = imp::Account::from_instance(self);
        self_.token.get().unwrap().clone()
    }

    pub fn token_id(&self) -> String {
        let self_ = imp::Account::from_instance(self);
        self_.token_id.borrow().clone()
    }

    pub fn otp_uri(&self) -> OTPUri {
        self.into()
    }

    pub fn qr_code(&self) -> QRCodeData {
        let otp: String = self.otp_uri().into();
        QRCodeData::from(otp.as_str())
    }

    pub fn set_name(&self, name: &str) -> Result<()> {
        let db = database::connection();
        let conn = db.get()?;

        let target = accounts::table.filter(accounts::columns::id.eq(self.id()));
        diesel::update(target)
            .set(accounts::columns::name.eq(name))
            .execute(&conn)?;

        self.set_property("name", &name)?;
        Ok(())
    }

    pub fn delete(&self) -> Result<()> {
        Keyring::remove_token(&self.token_id());
        let db = database::connection();
        let conn = db.get()?;
        diesel::delete(accounts::table.filter(accounts::columns::id.eq(&self.id())))
            .execute(&conn)?;
        Ok(())
    }
}
