use super::algorithm::Algorithm;
use crate::models::database;
use crate::models::{Account, FaviconError, FaviconScrapper};
use crate::schema::providers;
use anyhow::Result;
use diesel::QueryDsl;
use diesel::RunQueryDsl;
use gio::prelude::*;
use gio::FileExt;
use glib::subclass;
use glib::subclass::prelude::*;
use glib::translate::*;
use glib::Cast;
use glib::{StaticType, ToValue};
use gtk::FilterListModelExt;
use std::cell::{Cell, RefCell};
use std::str::FromStr;
use std::string::ToString;
use url::Url;

#[derive(Insertable)]
#[table_name = "providers"]
struct NewProvider {
    pub name: String,
    pub period: i32,
    pub algorithm: String,
    pub website: Option<String>,
    pub help_url: Option<String>,
    pub image_uri: Option<String>,
}

#[derive(Identifiable, Queryable, Associations, Hash, PartialEq, Eq, Debug, Clone)]
#[table_name = "providers"]
pub struct DiProvider {
    pub id: i32,
    pub name: String,
    pub period: i32,
    pub algorithm: String,
    pub website: Option<String>,
    pub help_url: Option<String>,
    pub image_uri: Option<String>,
}

pub struct ProviderPriv {
    pub id: Cell<i32>,
    pub name: RefCell<String>,
    pub period: Cell<i32>,
    pub algorithm: RefCell<String>,
    pub website: RefCell<Option<String>>,
    pub help_url: RefCell<Option<String>>,
    pub image_uri: RefCell<Option<String>>,
    pub accounts: gio::ListStore,
    pub filter_model: gtk::FilterListModel,
}

static PROPERTIES: [subclass::Property; 8] = [
    subclass::Property("id", |name| {
        glib::ParamSpec::int(name, "id", "Id", 0, 1000, 0, glib::ParamFlags::READWRITE)
    }),
    subclass::Property("name", |name| {
        glib::ParamSpec::string(name, "name", "Name", None, glib::ParamFlags::READWRITE)
    }),
    subclass::Property("accounts", |name| {
        glib::ParamSpec::object(
            name,
            "accounts",
            "accounts",
            gio::ListModel::static_type(),
            glib::ParamFlags::READWRITE,
        )
    }),
    subclass::Property("period", |name| {
        glib::ParamSpec::int(
            name,
            "period",
            "Period",
            0,
            1000,
            30,
            glib::ParamFlags::READWRITE,
        )
    }),
    subclass::Property("algorithm", |name| {
        glib::ParamSpec::string(
            name,
            "algorithm",
            "Algorithm",
            Some(&Algorithm::OTP.to_string()),
            glib::ParamFlags::READWRITE,
        )
    }),
    subclass::Property("website", |name| {
        glib::ParamSpec::string(
            name,
            "website",
            "Website",
            None,
            glib::ParamFlags::READWRITE,
        )
    }),
    subclass::Property("help-url", |name| {
        glib::ParamSpec::string(
            name,
            "help url",
            "Help URL",
            None,
            glib::ParamFlags::READWRITE,
        )
    }),
    subclass::Property("image-uri", |name| {
        glib::ParamSpec::string(
            name,
            "image uri",
            "Image URI",
            None,
            glib::ParamFlags::READWRITE,
        )
    }),
];

impl ObjectSubclass for ProviderPriv {
    const NAME: &'static str = "Provider";
    type ParentType = glib::Object;
    type Instance = subclass::simple::InstanceStruct<Self>;
    type Class = subclass::simple::ClassStruct<Self>;

    glib_object_subclass!();

    fn class_init(klass: &mut Self::Class) {
        klass.install_properties(&PROPERTIES);
    }

    fn new() -> Self {
        let model = gio::ListStore::new(Account::static_type());
        Self {
            id: Cell::new(0),
            name: RefCell::new("".to_string()),
            website: RefCell::new(None),
            help_url: RefCell::new(None),
            image_uri: RefCell::new(None),
            algorithm: RefCell::new(Algorithm::OTP.to_string()),
            period: Cell::new(30),
            filter_model: gtk::FilterListModel::new(Some(&model), gtk::NONE_FILTER),
            accounts: model,
        }
    }
}

impl ObjectImpl for ProviderPriv {
    fn set_property(&self, _obj: &glib::Object, id: usize, value: &glib::Value) {
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
            subclass::Property("period", ..) => {
                let period = value
                    .get_some()
                    .expect("type conformity checked by `Object::set_property`");
                self.period.replace(period);
            }
            subclass::Property("algorithm", ..) => {
                let algorithm = value
                    .get()
                    .expect("type conformity checked by `Object::set_property`")
                    .unwrap();
                self.algorithm.replace(algorithm);
            }
            subclass::Property("website", ..) => {
                let website = value
                    .get()
                    .expect("type conformity checked by `Object::set_property`");
                self.website.replace(website);
            }
            subclass::Property("help-url", ..) => {
                let help_url = value
                    .get()
                    .expect("type conformity checked by `Object::set_property`");
                self.help_url.replace(help_url);
            }
            subclass::Property("image-uri", ..) => {
                let image_uri = value
                    .get()
                    .expect("type conformity checked by `Object::set_property`");
                self.image_uri.replace(image_uri);
            }
            /*subclass::Property("accounts", ..) => {
                let accounts = value
                    .get()
                    .expect("type conformity checked by `Object::set_property`")
                    .unwrap();
                self.accounts.replace(accounts);
            }*/
            _ => unimplemented!(),
        }
    }

    fn get_property(&self, _obj: &glib::Object, id: usize) -> Result<glib::Value, ()> {
        let prop = &PROPERTIES[id];

        match *prop {
            subclass::Property("id", ..) => Ok(self.id.get().to_value()),
            subclass::Property("name", ..) => Ok(self.name.borrow().to_value()),
            subclass::Property("period", ..) => Ok(self.period.get().to_value()),
            subclass::Property("algorithm", ..) => Ok(self.algorithm.borrow().to_value()),
            subclass::Property("website", ..) => Ok(self.website.borrow().to_value()),
            subclass::Property("help-url", ..) => Ok(self.help_url.borrow().to_value()),
            subclass::Property("image-uri", ..) => Ok(self.image_uri.borrow().to_value()),
            //subclass::Property("accounts", ..) => Ok(self.accounts.borrow().to_value()),
            _ => unimplemented!(),
        }
    }
}
glib_wrapper! {
    pub struct Provider(Object<subclass::simple::InstanceStruct<ProviderPriv>, subclass::simple::ClassStruct<ProviderPriv>>);

    match fn {
        get_type => || ProviderPriv::get_type().to_glib(),
    }
}

impl Provider {
    pub fn create(
        name: &str,
        period: i32,
        algorithm: Algorithm,
        website: Option<String>,
    ) -> Result<Self> {
        use crate::diesel::ExpressionMethods;
        let db = database::connection();
        let conn = db.get()?;

        diesel::insert_into(providers::table)
            .values(NewProvider {
                name: name.to_string(),
                period,
                algorithm: algorithm.to_string(),
                website,
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

    pub fn load() -> Result<Vec<Self>> {
        use crate::schema::providers::dsl::*;
        let db = database::connection();
        let conn = db.get()?;

        let results = providers
            .load::<DiProvider>(&conn)?
            .into_iter()
            .map(From::from)
            .map(|p: Provider| {
                let accounts = Account::load(&p).unwrap();
                accounts.iter().for_each(|a| p.add_account(a));
                p
            })
            .collect::<Vec<Provider>>();

        Ok(results)
    }

    pub fn new(
        id: i32,
        name: &str,
        period: i32,
        algorithm: Algorithm,
        website: Option<String>,
        help_url: Option<String>,
        image_uri: Option<String>,
    ) -> Provider {
        glib::Object::new(
            Provider::static_type(),
            &[
                ("id", &id),
                ("name", &name),
                ("website", &website),
                ("help-url", &help_url),
                ("image-uri", &image_uri),
                ("period", &period),
                ("algorithm", &algorithm.to_string()),
            ],
        )
        .expect("Failed to create provider")
        .downcast()
        .expect("Created provider is of wrong type")
    }

    pub async fn favicon(&self) -> Result<gio::File, FaviconError> {
        let website_url = Url::parse(&self.website().unwrap())?;
        let favicons = FaviconScrapper::from_url(website_url).await?;

        let icon_name = format!("{}_{}", self.id(), self.name().replace(' ', "_"));
        let cache_path = glib::get_user_cache_dir()
            .join("authenticator")
            .join("favicons")
            .join(icon_name);
        let dest = gio::File::new_for_path(cache_path);

        if let Some(favicon) = favicons.get(0) {
            let mut res = surf::get(favicon).await?;
            let body = res.body_bytes().await?;
            dest.replace_contents(
                &body,
                None,
                false,
                gio::FileCreateFlags::REPLACE_DESTINATION,
                gio::NONE_CANCELLABLE,
            )?;
            return Ok(dest);
        }
        Err(FaviconError::NoResults)
    }

    pub fn id(&self) -> i32 {
        let priv_ = ProviderPriv::from_instance(self);
        priv_.id.get()
    }

    pub fn name(&self) -> String {
        let priv_ = ProviderPriv::from_instance(self);
        priv_.name.borrow().clone()
    }

    pub fn period(&self) -> i32 {
        let priv_ = ProviderPriv::from_instance(self);
        priv_.period.get()
    }

    pub fn algorithm(&self) -> Algorithm {
        let priv_ = ProviderPriv::from_instance(self);
        Algorithm::from_str(&priv_.algorithm.borrow().clone()).unwrap()
    }

    pub fn website(&self) -> Option<String> {
        let priv_ = ProviderPriv::from_instance(self);
        priv_.website.borrow().clone()
    }

    pub fn help_url(&self) -> Option<String> {
        let priv_ = ProviderPriv::from_instance(self);
        priv_.help_url.borrow().clone()
    }

    pub fn image_uri(&self) -> Option<String> {
        let priv_ = ProviderPriv::from_instance(self);
        priv_.image_uri.borrow().clone()
    }

    pub fn open_help(&self) {
        if let Some(ref url) = self.help_url() {
            gio::AppInfo::launch_default_for_uri(url, None::<&gio::AppLaunchContext>).unwrap();
        }
    }

    pub fn has_accounts(&self) -> bool {
        let priv_ = ProviderPriv::from_instance(self);
        priv_.accounts.get_n_items() != 0
    }

    pub fn add_account(&self, account: &Account) {
        let priv_ = ProviderPriv::from_instance(self);
        priv_.accounts.append(account);
    }

    pub fn accounts(&self) -> &gtk::FilterListModel {
        let priv_ = ProviderPriv::from_instance(self);
        &priv_.filter_model
    }

    pub fn search_accounts(&self, text: String) {
        let priv_ = ProviderPriv::from_instance(self);
        let filter = gtk::CustomFilter::new(Some(Box::new(move |obj| {
            let account = obj.downcast_ref::<Account>().unwrap();
            account.name().contains(&text)
        })));
        priv_.filter_model.set_filter(Some(&filter));
    }

    fn remove_account(&self, account: &Account) {}
}

impl From<DiProvider> for Provider {
    fn from(p: DiProvider) -> Self {
        Self::new(
            p.id,
            &p.name,
            p.period,
            Algorithm::from_str(&p.algorithm).unwrap(),
            p.website,
            p.help_url,
            p.image_uri,
        )
    }
}
