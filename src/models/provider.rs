use super::{
    algorithm::{Algorithm, OTPMethod},
    CLIENT,
};
use crate::{
    models::{database, otp, Account, AccountsModel, FaviconError, FaviconScrapper},
    schema::providers,
};
use anyhow::Result;
use async_std::prelude::*;
use core::cmp::Ordering;
use diesel::{ExpressionMethods, QueryDsl, RunQueryDsl};
use glib::{Cast, StaticType, ToValue};
use gtk::{gio, glib, prelude::*, subclass::prelude::*};
use std::{
    cell::{Cell, RefCell},
    str::FromStr,
    string::ToString,
};
use unicase::UniCase;
use url::Url;

#[derive(Insertable)]
#[table_name = "providers"]
struct NewProvider {
    pub name: String,
    pub website: Option<String>,
    pub help_url: Option<String>,
    pub image_uri: Option<String>,
    pub period: i32,
    pub digits: i32,
    pub default_counter: i32,
    pub algorithm: String,
    pub method: String,
}

#[derive(Identifiable, Queryable, Associations, Hash, PartialEq, Eq, Debug, Clone)]
#[table_name = "providers"]
pub struct DiProvider {
    pub id: i32,
    pub name: String,
    pub website: Option<String>,
    pub help_url: Option<String>,
    pub image_uri: Option<String>,
    pub period: i32,
    pub digits: i32,
    pub default_counter: i32,
    pub algorithm: String,
    pub method: String,
}
mod imp {
    use super::*;
    use glib::{subclass, ParamSpec};

    pub struct Provider {
        pub id: Cell<u32>,
        pub name: RefCell<String>,
        pub period: Cell<u32>,
        pub method: RefCell<String>,
        pub default_counter: Cell<u32>,
        pub algorithm: RefCell<String>,
        pub digits: Cell<u32>,
        pub website: RefCell<Option<String>>,
        pub help_url: RefCell<Option<String>>,
        pub image_uri: RefCell<Option<String>>,
        pub accounts: AccountsModel,
        pub filter_model: gtk::FilterListModel,
    }

    impl ObjectSubclass for Provider {
        const NAME: &'static str = "Provider";
        type Type = super::Provider;
        type ParentType = glib::Object;
        type Interfaces = ();
        type Instance = subclass::simple::InstanceStruct<Self>;
        type Class = subclass::simple::ClassStruct<Self>;

        glib::object_subclass!();
        fn new() -> Self {
            let model = AccountsModel::new();
            Self {
                id: Cell::new(0),
                default_counter: Cell::new(otp::HOTP_DEFAULT_COUNTER),
                algorithm: RefCell::new(Algorithm::default().to_string()),
                digits: Cell::new(otp::DEFAULT_DIGITS),
                name: RefCell::new("".to_string()),
                website: RefCell::new(None),
                help_url: RefCell::new(None),
                image_uri: RefCell::new(None),
                method: RefCell::new(OTPMethod::default().to_string()),
                period: Cell::new(otp::TOTP_DEFAULT_PERIOD),
                filter_model: gtk::FilterListModel::new(Some(&model), gtk::NONE_FILTER),
                accounts: model,
            }
        }
    }

    impl ObjectImpl for Provider {
        fn properties() -> &'static [ParamSpec] {
            use once_cell::sync::Lazy;
            static PROPERTIES: Lazy<Vec<ParamSpec>> = Lazy::new(|| {
                vec![
                    ParamSpec::uint(
                        "id",
                        "id",
                        "Id",
                        0,
                        u32::MAX,
                        0,
                        glib::ParamFlags::READWRITE,
                    ),
                    ParamSpec::string("name", "name", "Name", None, glib::ParamFlags::READWRITE),
                    ParamSpec::object(
                        "accounts",
                        "accounts",
                        "accounts",
                        AccountsModel::static_type(),
                        glib::ParamFlags::READWRITE,
                    ),
                    ParamSpec::uint(
                        "period",
                        "period",
                        "Period",
                        0,
                        1000,
                        otp::TOTP_DEFAULT_PERIOD,
                        glib::ParamFlags::READWRITE,
                    ),
                    ParamSpec::uint(
                        "digits",
                        "digits",
                        "Digits",
                        0,
                        1000,
                        otp::DEFAULT_DIGITS,
                        glib::ParamFlags::READWRITE,
                    ),
                    ParamSpec::uint(
                        "default-counter",
                        "default_counter",
                        "default_counter",
                        0,
                        u32::MAX,
                        otp::HOTP_DEFAULT_COUNTER,
                        glib::ParamFlags::READWRITE,
                    ),
                    ParamSpec::string(
                        "algorithm",
                        "algorithm",
                        "Algorithm",
                        Some(&Algorithm::default().to_string()),
                        glib::ParamFlags::READWRITE,
                    ),
                    ParamSpec::string(
                        "method",
                        "method",
                        "Method",
                        Some(&OTPMethod::default().to_string()),
                        glib::ParamFlags::READWRITE,
                    ),
                    ParamSpec::string(
                        "website",
                        "website",
                        "Website",
                        None,
                        glib::ParamFlags::READWRITE,
                    ),
                    ParamSpec::string(
                        "help-url",
                        "help url",
                        "Help URL",
                        None,
                        glib::ParamFlags::READWRITE,
                    ),
                    ParamSpec::string(
                        "image-uri",
                        "image uri",
                        "Image URI",
                        None,
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
            match pspec.get_name() {
                "id" => {
                    let id = value.get().unwrap().unwrap();
                    self.id.replace(id);
                }
                "name" => {
                    let name = value.get().unwrap().unwrap();
                    self.name.replace(name);
                }
                "period" => {
                    let period = value.get_some().unwrap();
                    self.period.replace(period);
                }
                "method" => {
                    let method = value.get().unwrap().unwrap();
                    self.method.replace(method);
                }
                "digits" => {
                    let digits = value.get_some().unwrap();
                    self.digits.replace(digits);
                }
                "algorithm" => {
                    let algorithm = value.get().unwrap().unwrap();
                    self.algorithm.replace(algorithm);
                }
                "default-counter" => {
                    let default_counter = value.get_some().unwrap();
                    self.default_counter.replace(default_counter);
                }
                "website" => {
                    let website = value.get().unwrap();
                    self.website.replace(website);
                }
                "help-url" => {
                    let help_url = value.get().unwrap();
                    self.help_url.replace(help_url);
                }
                "image-uri" => {
                    let image_uri = value.get().unwrap();
                    self.image_uri.replace(image_uri);
                }
                _ => unimplemented!(),
            }
        }

        fn get_property(&self, _obj: &Self::Type, _id: usize, pspec: &ParamSpec) -> glib::Value {
            match pspec.get_name() {
                "id" => self.id.get().to_value(),
                "name" => self.name.borrow().to_value(),
                "period" => self.period.get().to_value(),
                "method" => self.method.borrow().to_value(),
                "digits" => self.digits.get().to_value(),
                "algorithm" => self.algorithm.borrow().to_value(),
                "default-counter" => self.default_counter.get().to_value(),
                "website" => self.website.borrow().to_value(),
                "help-url" => self.help_url.borrow().to_value(),
                "image-uri" => self.image_uri.borrow().to_value(),
                "accounts" => self.accounts.to_value(),
                _ => unimplemented!(),
            }
        }
    }
}

glib::wrapper! {
    pub struct Provider(ObjectSubclass<imp::Provider>);
}

impl Provider {
    pub fn create(
        name: &str,
        period: u32,
        algorithm: Algorithm,
        website: Option<String>,
        method: OTPMethod,
        digits: u32,
        default_counter: u32,
    ) -> Result<Self> {
        let db = database::connection();
        let conn = db.get()?;

        diesel::insert_into(providers::table)
            .values(NewProvider {
                name: name.to_string(),
                period: period as i32,
                method: method.to_string(),
                website,
                algorithm: algorithm.to_string(),
                digits: digits as i32,
                default_counter: default_counter as i32,
                help_url: None,
                image_uri: None,
            })
            .execute(&conn)?;

        providers::table
            .order(providers::columns::id.desc())
            .first::<DiProvider>(&conn)
            .map_err(From::from)
            .map(From::from)
    }

    pub fn compare(obj1: &glib::Object, obj2: &glib::Object) -> Ordering {
        let provider1 = obj1.downcast_ref::<Provider>().unwrap();
        let provider2 = obj2.downcast_ref::<Provider>().unwrap();

        UniCase::new(provider1.name()).cmp(&UniCase::new(provider2.name()))
    }

    pub fn load() -> Result<impl Iterator<Item = Self>> {
        use crate::schema::providers::dsl::*;
        let db = database::connection();
        let conn = db.get()?;

        let results = providers
            .load::<DiProvider>(&conn)?
            .into_iter()
            .map(From::from)
            .map(|p: Provider| {
                Account::load(&p).unwrap().for_each(|a| p.add_account(&a));
                p
            });

        Ok(results)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: u32,
        name: &str,
        period: u32,
        method: OTPMethod,
        algorithm: Algorithm,
        digits: u32,
        default_counter: u32,
        website: Option<String>,
        help_url: Option<String>,
        image_uri: Option<String>,
    ) -> Provider {
        glib::Object::new(&[
            ("id", &id),
            ("name", &name),
            ("website", &website),
            ("help-url", &help_url),
            ("image-uri", &image_uri),
            ("period", &period),
            ("method", &method.to_string()),
            ("algorithm", &algorithm.to_string()),
            ("digits", &digits),
            ("default-counter", &default_counter),
        ])
        .expect("Failed to create provider")
    }

    pub async fn favicon(&self) -> Result<gio::File, Box<dyn std::error::Error>> {
        if let Some(ref website) = self.website() {
            let website_url = Url::parse(website)?;
            let favicon = FaviconScrapper::from_url(website_url).await?;

            let icon_name = format!("{}_{}", self.id(), self.name().replace(' ', "_"));
            let cache_path = glib::get_user_cache_dir()
                .join("authenticator")
                .join("favicons")
                .join(icon_name);
            let mut dest = async_std::fs::File::create(cache_path.clone()).await?;

            if let Some(best_favicon) = favicon.find_best().await {
                let mut res = CLIENT.get(best_favicon).await?;
                let body = res.body_bytes().await?;
                dest.write_all(&body).await?;

                return Ok(gio::File::new_for_path(cache_path));
            }
        }
        Err(Box::new(FaviconError::NoResults))
    }

    pub fn id(&self) -> u32 {
        let self_ = imp::Provider::from_instance(self);
        self_.id.get()
    }

    pub fn name(&self) -> String {
        let self_ = imp::Provider::from_instance(self);
        self_.name.borrow().clone()
    }

    pub fn digits(&self) -> u32 {
        let self_ = imp::Provider::from_instance(self);
        self_.digits.get()
    }

    pub fn default_counter(&self) -> u32 {
        let self_ = imp::Provider::from_instance(self);
        self_.default_counter.get()
    }

    pub fn period(&self) -> u32 {
        let self_ = imp::Provider::from_instance(self);
        self_.period.get()
    }

    pub fn algorithm(&self) -> Algorithm {
        let self_ = imp::Provider::from_instance(self);
        Algorithm::from_str(&self_.algorithm.borrow().clone()).unwrap()
    }

    pub fn method(&self) -> OTPMethod {
        let self_ = imp::Provider::from_instance(self);
        OTPMethod::from_str(&self_.method.borrow().clone()).unwrap()
    }

    pub fn website(&self) -> Option<String> {
        let self_ = imp::Provider::from_instance(self);
        self_.website.borrow().clone()
    }

    pub fn help_url(&self) -> Option<String> {
        let self_ = imp::Provider::from_instance(self);
        self_.help_url.borrow().clone()
    }

    pub fn set_image_uri(&self, uri: &str) -> Result<()> {
        let db = database::connection();
        let conn = db.get()?;

        let target = providers::table.filter(providers::columns::id.eq(self.id() as i32));
        diesel::update(target)
            .set(providers::columns::image_uri.eq(uri))
            .execute(&conn)?;

        self.set_property("image-uri", &uri)?;
        Ok(())
    }

    pub fn image_uri(&self) -> Option<String> {
        let self_ = imp::Provider::from_instance(self);
        self_.image_uri.borrow().clone()
    }

    pub fn open_help(&self) {
        if let Some(ref url) = self.help_url() {
            gio::AppInfo::launch_default_for_uri(url, None::<&gio::AppLaunchContext>).unwrap();
        }
    }

    pub fn has_account(&self, account: &Account) -> Option<u32> {
        let self_ = imp::Provider::from_instance(self);
        self_.accounts.find_by_id(account.id())
    }

    pub fn has_accounts(&self) -> bool {
        let self_ = imp::Provider::from_instance(self);
        self_.accounts.get_n_items() != 0
    }

    pub fn add_account(&self, account: &Account) {
        let self_ = imp::Provider::from_instance(self);
        self_.accounts.insert(account);
    }

    pub fn accounts_model(&self) -> &AccountsModel {
        let self_ = imp::Provider::from_instance(self);
        &self_.accounts
    }

    pub fn accounts(&self) -> &gtk::FilterListModel {
        let self_ = imp::Provider::from_instance(self);
        &self_.filter_model
    }

    pub fn filter(&self, text: String) {
        let self_ = imp::Provider::from_instance(self);
        let filter = gtk::CustomFilter::new(glib::clone!(@weak self as provider => move |obj| {
            let account = obj.downcast_ref::<Account>().unwrap();
            let query = &text.to_ascii_lowercase();
            let provider_match = provider.name().to_ascii_lowercase().contains(query);
            account
                .name()
                .to_ascii_lowercase()
                .contains(query) || provider_match
        }));
        self_.filter_model.set_filter(Some(&filter));
    }

    pub fn remove_account(&self, account: Account) {
        let self_ = imp::Provider::from_instance(self);
        if let Some(pos) = self_.accounts.find_by_id(account.id()) {
            self_.accounts.remove(pos);
        }
    }
}

impl From<DiProvider> for Provider {
    fn from(p: DiProvider) -> Self {
        Self::new(
            p.id as u32,
            &p.name,
            p.period as u32,
            OTPMethod::from_str(&p.method).unwrap(),
            Algorithm::from_str(&p.algorithm).unwrap(),
            p.digits as u32,
            p.default_counter as u32,
            p.website,
            p.help_url,
            p.image_uri,
        )
    }
}

impl From<&Provider> for DiProvider {
    fn from(p: &Provider) -> Self {
        Self {
            id: p.id() as i32,
            name: p.name(),
            period: p.period() as i32,
            method: p.method().to_string(),
            algorithm: p.algorithm().to_string(),
            digits: p.digits() as i32,
            default_counter: p.default_counter() as i32,
            website: p.website(),
            help_url: p.help_url(),
            image_uri: p.image_uri(),
        }
    }
}
