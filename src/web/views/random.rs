use poem::web::{Data, Redirect};
use poem::{handler, IntoResponse};
use rand::prelude::*;

use crate::prelude::*;
use crate::{database, wargaming};

#[handler]
#[instrument(skip_all, level = "info")]
pub async fn get_random(db: Data<&mongodb::Database>) -> Result<impl IntoResponse> {
    let realm = [wargaming::Realm::Russia, wargaming::Realm::Europe]
        .choose(&mut thread_rng())
        .unwrap();
    let account = database::Account::sample_account(*db, *realm).await?;
    Ok(Redirect::temporary(format!("/{}/{}", realm.to_str(), account.id)))
}
