use core::cmp::Ordering;
use std::cell::{Cell, RefCell};

use anyhow::{Context, Result};
use diesel::{
    Associations, BelongingToDsl, ExpressionMethods, Identifiable, Insertable, QueryDsl, Queryable,
    RunQueryDsl,
};
use gtk::{
    glib::{self, clone},
    prelude::*,
    subclass::prelude::*,
};
use once_cell::sync::OnceCell;
use unicase::UniCase;

use super::{
    provider::{DiProvider, Provider},
    OTPMethod, OTPUri, RUNTIME,
};
use crate::{
    models::{database, keyring, otp},
    schema::accounts,
    utils::spawn_tokio_blocking,
    widgets::QRCodeData,
};

#[derive(Insertable)]
#[diesel(table_name = accounts)]
struct NewAccount {
    pub name: String,
    pub token_id: String,
    pub provider_id: i32,
    pub counter: i32,
}

#[derive(Identifiable, Queryable, Associations, Hash, PartialEq, Eq, Debug, Clone)]
#[diesel(belongs_to(DiProvider, foreign_key = provider_id))]
#[diesel(table_name = accounts)]
pub struct DiAccount {
    pub id: i32,
    pub name: String,
    pub counter: i32,
    pub token_id: String,
    pub provider_id: i32,
}

#[doc(hidden)]
mod imp {
    use glib::{ParamFlags, ParamSpec, ParamSpecObject, ParamSpecString, ParamSpecUInt, Value};
    use once_cell::sync::Lazy;

    use super::*;

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
                id: Cell::default(),
                counter: Cell::new(otp::HOTP_DEFAULT_COUNTER),
                name: RefCell::default(),
                otp: RefCell::default(),
                token_id: RefCell::default(),
                provider: RefCell::default(),
                token: OnceCell::default(),
            }
        }
    }

    impl ObjectImpl for Account {
        fn properties() -> &'static [ParamSpec] {
            static PROPERTIES: Lazy<Vec<ParamSpec>> = Lazy::new(|| {
                vec![
                    ParamSpecUInt::new(
                        "id",
                        "",
                        "",
                        0,
                        u32::MAX,
                        0,
                        ParamFlags::READWRITE | ParamFlags::CONSTRUCT,
                    ),
                    ParamSpecUInt::new(
                        "counter",
                        "",
                        "",
                        0,
                        u32::MAX,
                        otp::HOTP_DEFAULT_COUNTER,
                        ParamFlags::READWRITE,
                    ),
                    ParamSpecString::builder("name").build(),
                    ParamSpecString::builder("token-id").build(),
                    ParamSpecString::builder("otp").build(),
                    ParamSpecObject::builder::<Provider>("provider").build(),
                ]
            });
            PROPERTIES.as_ref()
        }

        fn set_property(&self, _id: usize, value: &Value, pspec: &ParamSpec) {
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

        fn property(&self, _id: usize, pspec: &ParamSpec) -> Value {
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
    pub fn create(
        name: &str,
        token: &str,
        counter: Option<u32>,
        provider: &Provider,
    ) -> Result<Account> {
        let db = database::connection();
        let mut conn = db.get()?;

        let label = format!("{} - {name}", provider.name());
        let token_send = token.to_owned();
        let token_id = spawn_tokio_blocking(async move {
            keyring::store(&label, &token_send)
                .await
                .context("Failed to save token")
        })?;

        diesel::insert_into(accounts::table)
            .values(NewAccount {
                name: name.to_string(),
                token_id,
                provider_id: provider.id() as i32,
                counter: counter.unwrap_or_else(|| provider.default_counter()) as i32,
            })
            .execute(&mut conn)?;

        accounts::table
            .order(accounts::columns::id.desc())
            .first::<DiAccount>(&mut conn)
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
        let mut conn = db.get()?;

        let dip: DiProvider = p.into();
        let results = DiAccount::belonging_to(&dip)
            .load::<DiAccount>(&mut conn)?
            .into_iter()
            .filter_map(clone!(@strong p => move |account| {
                match Self::new(
                    account.id  as u32,
                    &account.name,
                    &account.token_id,
                    account.counter as u32,
                    p.clone(),
                    None,
                )
                {
                    Ok(account) => Some(account),
                    Err(e) => {
                        let name = account.name;
                        let provider = p.name();
                        tracing::error!("Failed to load account '{name}' / '{provider}' with error {e}");
                        None
                    }
                }
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
        let account = glib::Object::new::<Self>(&[
            ("id", &id),
            ("name", &name),
            ("token-id", &token_id),
            ("provider", &provider),
            ("counter", &counter),
        ]);

        let token = if let Some(t) = token {
            t.to_string()
        } else {
            let token_id = token_id.to_owned();
            spawn_tokio_blocking(async move {
                keyring::token(&token_id).await?.with_context(|| {
                    format!("Could not get item with token identifier '{token_id}' from keyring")
                })
            })?
        };
        account.imp().token.set(token).unwrap();
        account.generate_otp();
        Ok(account)
    }

    pub fn generate_otp(&self) {
        let provider = self.provider();

        let counter = match provider.method() {
            OTPMethod::TOTP => otp::time_based_counter(provider.period()),
            OTPMethod::HOTP => self.counter() as u64,
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
                tracing::warn!("Failed to generate the OTP {}", err);
                "Error".to_string()
            }
        };

        self.set_property("otp", &label);
    }

    /// Increment the internal counter in case of a HOTP account
    pub fn increment_counter(&self) -> Result<()> {
        let new_value = self.counter() + 1;
        self.imp().counter.set(new_value);

        let db = database::connection();
        let mut conn = db.get()?;

        let target = accounts::table.filter(accounts::columns::id.eq(self.id() as i32));
        diesel::update(target)
            .set(accounts::columns::counter.eq(new_value as i32))
            .execute(&mut conn)?;
        Ok(())
    }

    pub fn otp(&self) -> String {
        self.property("otp")
    }

    pub fn copy_otp(&self) {
        let display = gtk::gdk::Display::default().unwrap();
        let clipboard = display.clipboard();
        // The codes come with the white space shown in the label.
        let code = &self.imp().otp.borrow().replace(' ', "");
        clipboard.set_text(code);

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

    pub fn set_provider(&self, provider: &Provider) -> Result<()> {
        let db = database::connection();
        let mut conn = db.get()?;

        let target = accounts::table.filter(accounts::columns::id.eq(self.id() as i32));
        diesel::update(target)
            .set(accounts::columns::provider_id.eq(provider.id() as i32))
            .execute(&mut conn)?;

        self.set_property("provider", provider);
        Ok(())
    }

    pub fn counter(&self) -> u32 {
        self.imp().counter.get()
    }

    pub fn name(&self) -> String {
        self.imp().name.borrow().clone()
    }

    pub fn connect_name_notify<F>(&self, callback: F) -> glib::SignalHandlerId
    where
        F: Fn(&Self, String) + 'static,
    {
        self.connect_notify_local(
            Some("name"),
            clone!(@weak self as app => move |_, _| {
                let name = app.name();
                callback(&app, name);
            }),
        )
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
        let mut conn = db.get()?;

        let target = accounts::table.filter(accounts::columns::id.eq(self.id() as i32));
        diesel::update(target)
            .set(accounts::columns::name.eq(name))
            .execute(&mut conn)?;

        self.set_property("name", &name);
        Ok(())
    }

    pub fn set_counter(&self, counter: u32) -> Result<()> {
        let db = database::connection();
        let mut conn = db.get()?;

        let target = accounts::table.filter(accounts::columns::id.eq(self.id() as i32));
        diesel::update(target)
            .set(accounts::columns::counter.eq(counter as i32))
            .execute(&mut conn)?;

        self.set_property("counter", &counter);
        Ok(())
    }

    pub fn delete(&self) -> Result<()> {
        let token_id = self.token_id();
        RUNTIME.spawn(async move {
            if let Err(err) = keyring::remove_token(&token_id).await {
                tracing::error!("Failed to remove the token from secret service {}", err);
            }
        });
        let db = database::connection();
        let mut conn = db.get()?;
        diesel::delete(accounts::table.filter(accounts::columns::id.eq(self.id() as i32)))
            .execute(&mut conn)?;
        Ok(())
    }
}
