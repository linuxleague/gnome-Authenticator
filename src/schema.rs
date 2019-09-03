table! {
    accounts (id) {
        id -> Integer,
        username -> Text,
        token_id -> Text,
        provider -> Integer,
    }
}

table! {
    providers (id) {
        id -> Integer,
        name -> Text,
        website -> Text,
        help_url -> Text,
        image_uri -> Text,
    }
}
