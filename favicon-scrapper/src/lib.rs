use std::sync::OnceLock;

static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
pub(crate) fn client<'a>() -> &'a reqwest::Client {
    CLIENT.get_or_init(|| reqwest::Client::new())
}

mod error;
mod favicon;
mod format;
mod metadata;
mod scrapper;

pub use error::Error;
pub use favicon::Favicon;
pub use format::Format;
pub use metadata::Metadata;
pub use scrapper::Scrapper;

#[cfg(test)]
mod tests {
    use url::Url;

    use super::*;

    #[tokio::test]
    async fn parse_from_file() {
        let base_url = Url::parse("https://github.com").unwrap();
        let expected_output = Favicon::for_url(
            "https://github.githubassets.com/favicon.ico",
            Metadata::new(Format::Ico),
        );

        let scrapper = Scrapper::from_file(
            "./tests/parser/url_shortcut_icon_link.html".into(),
            Some(&base_url),
        )
        .await
        .unwrap();
        let best = scrapper.find_best().await;
        assert_eq!(best, Some(&expected_output));

        let scrapper =
            Scrapper::from_file("./tests/parser/url_icon_link.html".into(), Some(&base_url))
                .await
                .unwrap();
        let best = scrapper.find_best().await;
        assert_eq!(best, Some(&expected_output));

        let scrapper =
            Scrapper::from_file("./tests/parser/url_fluid_icon.html".into(), Some(&base_url))
                .await
                .unwrap();
        let best = scrapper.find_best().await;
        assert_eq!(best, Some(&expected_output));

        let scrapper = Scrapper::from_file(
            "./tests/parser/url_apple_touch_icon_precomposed_link.html".into(),
            Some(&base_url),
        )
        .await
        .unwrap();
        let best = scrapper.find_best().await;
        assert_eq!(best, Some(&expected_output));

        let scrapper = Scrapper::from_file(
            "./tests/parser/url_apple_touch_icon.html".into(),
            Some(&base_url),
        )
        .await
        .unwrap();
        let best = scrapper.find_best().await;
        assert_eq!(best, Some(&expected_output));

        let base_url = Url::parse("https://gitlab.com").unwrap();
        let expected_output = Favicon::for_url("https://assets.gitlab-static.net/assets/msapplication-tile-1196ec67452f618d39cdd85e2e3a542f76574c071051ae7effbfde01710eb17d.png", Metadata::new(Format::Png));
        let scrapper = Scrapper::from_file("./tests/parser/meta_tag.html".into(), Some(&base_url))
            .await
            .unwrap();
        let best = scrapper.find_best().await;
        assert_eq!(best, Some(&expected_output));

        let base_url = Url::parse("http://127.0.0.1:8000/index.html").unwrap();
        let expected_output = Favicon::for_url(
            "http://127.0.0.1:8000/favicon.ico",
            Metadata::new(Format::Ico),
        );
        let scrapper =
            Scrapper::from_file("./tests/parser/url_with_port.html".into(), Some(&base_url))
                .await
                .unwrap();
        let best = scrapper.find_best().await;
        assert_eq!(best, Some(&expected_output));
    }

    #[tokio::test]
    async fn parse_data_base64() {
        let scrapper = Scrapper::from_file("./tests/parser/data_base64.html".into(), None)
            .await
            .unwrap();
        assert!(!scrapper.is_empty());
        assert_eq!(scrapper.len(), 1);
        let best = scrapper.find_best().await.unwrap();

        assert!(best.metadata().format().is_ico());
        assert!(best.is_data());
        assert_eq!(best.size().await, Some((16, 16)));
    }

    #[tokio::test]
    async fn parse_data_svg() {
        let scrapper = Scrapper::from_file("./tests/parser/data_svg.html".into(), None)
            .await
            .unwrap();
        assert!(!scrapper.is_empty());
        assert_eq!(scrapper.len(), 1);
        let best = scrapper.find_best().await.unwrap();

        assert!(best.metadata().format().is_svg());
        assert!(best.is_data());
    }

    #[tokio::test]
    async fn parse_sizes() {
        let base_url = Url::parse("https://about.gitlab.com").ok();
        let scrapper = Scrapper::from_file("./tests/parser/size.html".into(), base_url)
            .await
            .unwrap();
        assert!(!scrapper.is_empty());
        // There are 16 but we always add the favicon.ico to try in case it exists as
        // well
        assert_eq!(scrapper.len(), 16 + 1);

        assert_eq!(
            scrapper[0],
            Favicon::for_url(
                "https://about.gitlab.com/nuxt-images/ico/mstile-144x144.png?cache=20220413",
                Metadata::new(Format::Png)
            )
        );
        assert_eq!(
            scrapper[1],
            Favicon::for_url(
                "https://about.gitlab.com/nuxt-images/ico/favicon.ico?cache=20220413",
                Metadata::new(Format::Ico),
            )
        );
        assert_eq!(
            scrapper[2],
            Favicon::for_url(
                "https://about.gitlab.com/nuxt-images/ico/favicon-192x192.png?cache=2022041",
                Metadata::with_size(Format::Png, (192, 192)),
            )
        );
        assert_eq!(
            scrapper[3],
            Favicon::for_url(
                "https://about.gitlab.com/nuxt-images/ico/favicon-160x160.png?cache=2022041",
                Metadata::with_size(Format::Png, (160, 160))
            )
        );
        assert_eq!(
            scrapper[4],
            Favicon::for_url(
                "https://about.gitlab.com/nuxt-images/ico/favicon-96x96.png?cache=2022041",
                Metadata::with_size(Format::Png, (96, 96))
            )
        );
        assert_eq!(
            scrapper[5],
            Favicon::for_url(
                "https://about.gitlab.com/nuxt-images/ico/favicon-32x32.png?cache=2022041",
                Metadata::with_size(Format::Png, (32, 32))
            )
        );
        assert_eq!(
            scrapper[6],
            Favicon::for_url(
                "https://about.gitlab.com/nuxt-images/ico/favicon-16x16.png?cache=2022041",
                Metadata::with_size(Format::Png, (16, 16))
            )
        );
        assert_eq!(
            scrapper[7],
            Favicon::for_url(
                "https://about.gitlab.com/nuxt-images/ico/apple-touch-icon-57x57.png?cache=2022041",
                Metadata::with_size(Format::Png, (57, 57))
            )
        );
        assert_eq!(
            scrapper[8],
            Favicon::for_url(
                "https://about.gitlab.com/nuxt-images/ico/apple-touch-icon-60x60.png?cache=2022041",
                Metadata::with_size(Format::Png, (60, 60))
            )
        );
        assert_eq!(
            scrapper[9],
            Favicon::for_url(
                "https://about.gitlab.com/nuxt-images/ico/apple-touch-icon-72x72.png?cache=2022041",
                Metadata::with_size(Format::Png, (72, 72))
            )
        );
        assert_eq!(
            scrapper[10],
            Favicon::for_url(
                "https://about.gitlab.com/nuxt-images/ico/apple-touch-icon-76x76.png?cache=2022041",
                Metadata::with_size(Format::Png, (76, 76))
            )
        );
        assert_eq!(scrapper[11], Favicon::for_url("https://about.gitlab.com/nuxt-images/ico/apple-touch-icon-114x114.png?cache=2022041", Metadata::with_size(Format::Png, (114, 114 ))));
        assert_eq!(scrapper[12], Favicon::for_url("https://about.gitlab.com/nuxt-images/ico/apple-touch-icon-120x120.png?cache=2022041", Metadata::with_size(Format::Png, (120, 120 ))));
        assert_eq!(scrapper[13], Favicon::for_url("https://about.gitlab.com/nuxt-images/ico/apple-touch-icon-144x144.png?cache=2022041", Metadata::with_size(Format::Png, (144, 144 ))));
        assert_eq!(scrapper[14], Favicon::for_url("https://about.gitlab.com/nuxt-images/ico/apple-touch-icon-152x152.png?cache=2022041", Metadata::with_size(Format::Png, (152, 152 ))));
        assert_eq!(scrapper[15], Favicon::for_url("https://about.gitlab.com/nuxt-images/ico/apple-touch-icon-180x180.png?cache=2022041", Metadata::with_size(Format::Png, (180, 180 ))));
    }
}
