use super::{
    provider::{DiProvider, Provider},
    OTPMethod, OTPUri,
};
use crate::{
    models::{database, otp, Keyring},
    schema::accounts,
    widgets::QRCodeData,
};
use anyhow::{Context, Result};
use core::cmp::Ordering;
use diesel::{BelongingToDsl, ExpressionMethods, QueryDsl, RunQueryDsl};
use glib::{clone, Cast, StaticType, ToValue};
use gtk::{glib, prelude::*, subclass::prelude::*};
use once_cell::sync::OnceCell;
use std::cell::{Cell, RefCell};
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
    use glib::ParamSpec;

    pub struct Account {
        pub id: Cell<u32>,
        pub otp: RefCell<String>,
        pub name: RefCell<String>,
        pub counter: Cell<u32>,
        pub token: OnceCell<String>,
        pub token_id: RefCell<String>,
        pub provider: RefCell<Option<Provider>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Account {
        const NAME: &'static str = "Account";
        type Type = super::Account;

        fn new() -> Self {
            Self {
                id: Cell::new(0),
                counter: Cell::new(otp::HOTP_DEFAULT_COUNTER),
                name: RefCell::new("".to_string()),
                otp: RefCell::new("".to_string()),
                token_id: RefCell::new("".to_string()),
                provider: RefCell::new(None),
                token: OnceCell::new(),
            }
        }
    }

    impl ObjectImpl for Account {
        fn properties() -> &'static [ParamSpec] {
            use once_cell::sync::Lazy;

            static PROPERTIES: Lazy<Vec<ParamSpec>> = Lazy::new(|| {
                vec![
                    ParamSpec::new_uint(
                        "id",
                        "id",
                        "Id",
                        0,
                        u32::MAX,
                        0,
                        glib::ParamFlags::READWRITE,
                    ),
                    ParamSpec::new_uint(
                        "counter",
                        "counter",
                        "Counter",
                        0,
                        u32::MAX,
                        otp::HOTP_DEFAULT_COUNTER,
                        glib::ParamFlags::READWRITE,
                    ),
                    ParamSpec::new_string(
                        "name",
                        "name",
                        "Name",
                        None,
                        glib::ParamFlags::READWRITE,
                    ),
                    ParamSpec::new_string(
                        "token-id",
                        "token-id",
                        "token id",
                        None,
                        glib::ParamFlags::READWRITE,
                    ),
                    ParamSpec::new_string(
                        "otp",
                        "otp",
                        "The One Time Password",
                        None,
                        glib::ParamFlags::READWRITE,
                    ),
                    ParamSpec::new_object(
                        "provider",
                        "provider",
                        "The account provider",
                        Provider::static_type(),
                        glib::ParamFlags::READWRITE,
                    ),
                ]
            });
            PROPERTIES.as_ref()
        }

        fn set_property(
            &self,
            _obj: &Self::Type,
            _id: usize,
            value: &glib::Value,
            pspec: &ParamSpec,
        ) {
            match pspec.name() {
                "id" => {
                    let id = value.get().unwrap();
                    self.id.replace(id);
                }
                "name" => {
                    let name = value.get().unwrap();
                    self.name.replace(name);
                }
                "counter" => {
                    let counter = value.get().unwrap();
                    self.counter.replace(counter);
                }
                "otp" => {
                    let otp = value.get().unwrap();
                    self.otp.replace(otp);
                }
                "token-id" => {
                    let token_id = value.get().unwrap();
                    self.token_id.replace(token_id);
                }
                "provider" => {
                    let provider = value.get().unwrap();
                    self.provider.replace(provider);
                }
                _ => unimplemented!(),
            }
        }

        fn property(&self, _obj: &Self::Type, _id: usize, pspec: &ParamSpec) -> glib::Value {
            match pspec.name() {
                "id" => self.id.get().to_value(),
                "name" => self.name.borrow().to_value(),
                "counter" => self.counter.get().to_value(),
                "otp" => self.otp.borrow().to_value(),
                "token-id" => self.token_id.borrow().to_value(),
                "provider" => self.provider.borrow().to_value(),
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
            .context("Failed to save token")?;

        diesel::insert_into(accounts::table)
            .values(NewAccount {
                name: name.to_string(),
                token_id,
                provider_id: provider.id() as i32,
                counter: provider.default_counter() as i32,
            })
            .execute(&conn)?;

        accounts::table
            .order(accounts::columns::id.desc())
            .first::<DiAccount>(&conn)
            .map_err(From::from)
            .map(|account| {
                Self::new(
                    account.id as u32,
                    &account.name,
                    &account.token_id,
                    account.counter as u32,
                    provider.clone(),
                    Some(token),
                )
                .unwrap()
            })
    }

    pub fn load(p: &Provider) -> Result<impl Iterator<Item = Self>> {
        let db = database::connection();
        let conn = db.get()?;

        let dip: DiProvider = p.into();
        let results = DiAccount::belonging_to(&dip)
            .load::<DiAccount>(&conn)?
            .into_iter()
            .filter_map(clone!(@strong p => move |account| {
                Self::new(
                    account.id  as u32,
                    &account.name,
                    &account.token_id,
                    account.counter as u32,
                    p.clone(),
                    None,
                ).ok()
            }));

        Ok(results)
    }

    pub fn compare(obj1: &glib::Object, obj2: &glib::Object) -> Ordering {
        let account1 = obj1.downcast_ref::<Account>().unwrap();
        let account2 = obj2.downcast_ref::<Account>().unwrap();

        UniCase::new(account1.name()).cmp(&UniCase::new(account2.name()))
    }

    pub fn new(
        id: u32,
        name: &str,
        token_id: &str,
        counter: u32,
        provider: Provider,
        token: Option<&str>,
    ) -> Result<Account> {
        let account = glib::Object::new(&[
            ("id", &id),
            ("name", &name),
            ("token-id", &token_id),
            ("provider", &provider),
            ("counter", &counter),
        ])
        .context("Failed to create account")?;

        let token = if let Some(t) = token {
            t.to_string()
        } else {
            Keyring::token(token_id)?.context("Could not get item from keyring")?
        };

        account.imp().token.set(token).unwrap();
        Ok(account)
    }

    pub fn generate_otp(&self) {
        let provider = self.provider();

        let counter = match provider.method() {
            OTPMethod::TOTP => otp::time_based_counter(provider.period()),
            OTPMethod::HOTP => {
                let old_counter = self.counter();
                if let Err(err) = self.increment_counter() {
                    error!("Failed to increment HOTP counter {}", err);
                }
                old_counter as u64
            }
            OTPMethod::Steam => otp::time_based_counter(otp::STEAM_DEFAULT_PERIOD),
        };

        let otp_password: Result<String> = match provider.method() {
            OTPMethod::Steam => otp::steam(&self.token(), counter),
            _ => {
                let token = otp::hotp(
                    &self.token(),
                    counter,
                    provider.algorithm(),
                    provider.digits() as u32,
                );

                token.map(|d| otp::format(d, provider.digits() as usize))
            }
        };

        let label = match otp_password {
            Ok(password) => password,
            Err(err) => {
                warn!("Failed to generate the OTP {}", err);
                "Error".to_string()
            }
        };

        self.set_property("otp", &label);
    }

    fn increment_counter(&self) -> Result<()> {
        // For security reasons, never re-use the same counter for HOTP
        let new_value = self.counter() + 1;
        self.imp().counter.set(new_value);

        let db = database::connection();
        let conn = db.get()?;

        let target = accounts::table.filter(accounts::columns::id.eq(self.id() as i32));
        diesel::update(target)
            .set(accounts::columns::counter.eq(new_value as i32))
            .execute(&conn)?;
        Ok(())
    }

    pub fn copy_otp(&self) {
        let display = gtk::gdk::Display::default().unwrap();
        let clipboard = display.clipboard();
        // The codes come with the white space shown in the label.
        let code = &self.imp().otp.borrow().replace(' ', "");
        clipboard.set_text(&code);

        // Indirectly increment the counter once the token was copied
        if self.provider().method() == OTPMethod::HOTP {
            self.generate_otp();
        }
    }

    pub fn id(&self) -> u32 {
        self.imp().id.get()
    }

    pub fn provider(&self) -> Provider {
        self.property("provider")
    }

    pub fn counter(&self) -> u32 {
        self.imp().counter.get()
    }

    pub fn name(&self) -> String {
        self.imp().name.borrow().clone()
    }

    pub fn token(&self) -> String {
        self.imp().token.get().unwrap().clone()
    }

    pub fn token_id(&self) -> String {
        self.imp().token_id.borrow().clone()
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

        let target = accounts::table.filter(accounts::columns::id.eq(self.id() as i32));
        diesel::update(target)
            .set(accounts::columns::name.eq(name))
            .execute(&conn)?;

        self.set_property("name", &name);
        Ok(())
    }

    pub fn delete(&self) -> Result<()> {
        let token_id = self.token_id();
        std::thread::spawn(move || {
            if let Err(err) = Keyring::remove_token(&token_id) {
                error!("Failed to remove the token from secret service {}", err);
            }
        });
        let db = database::connection();
        let conn = db.get()?;
        diesel::delete(accounts::table.filter(accounts::columns::id.eq(self.id() as i32)))
            .execute(&conn)?;
        Ok(())
    }
}
