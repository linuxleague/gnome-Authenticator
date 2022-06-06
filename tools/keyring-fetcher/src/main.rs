use std::{collections::HashMap, path::PathBuf};

#[async_std::main]
async fn main() -> oo7::Result<()> {
    let home = std::env::var("HOME").unwrap();
    let keyring_path = [
        &home,
        ".var/app/com.belmoussaoui.Authenticator/data/keyrings/default.keyring",
    ]
    .iter()
    .collect::<PathBuf>();

    let host_keyring = oo7::dbus::Service::new(oo7::dbus::Algorithm::Encrypted).await?;
    let collection = host_keyring
        .with_alias("login")
        .await?
        .expect("'login' collection not found");

    let items = collection
        .search_items(HashMap::from([(
            "app_id",
            "com.belmoussaoui.Authenticator",
        )]))
        .await?;

    let secret = items[0].secret().await?;
    let keyring = oo7::portal::Keyring::load(keyring_path, &secret).await?;

    let keyring_items = keyring.items().await?;
    for item in keyring_items.iter() {
        let attributes = item.attributes();
        let secret = item.secret();
        println!(
            "Found a secret: \nAttributes: {:#?}\nSecret: {:#?}",
            attributes,
            String::from_utf8_lossy(&secret)
        );
        println!("################################################");
    }

    Ok(())
}
