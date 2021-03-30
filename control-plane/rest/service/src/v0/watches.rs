use super::*;
use std::convert::TryFrom;

pub(super) fn configure(cfg: &mut paperclip::actix::web::ServiceConfig) {
    cfg.service(put_watch)
        .service(del_watch)
        .service(get_watches);
}

#[put("/watches/volume/{volume_id}", tags(Watches))]
async fn put_watch(
    web::Path(volume_id): web::Path<VolumeId>,
    web::Query(watch): web::Query<WatchTypeQueryParam>,
) -> Result<Json<()>, RestError> {
    CreateWatch {
        resource: WatchResource::Volume(volume_id),
        callback: WatchCallback::Uri(watch.callback.to_string()),
        watch_type: WatchType::Actual,
    }
    .request()
    .await?;

    Ok(Json(()))
}

#[get("/watches/volume/{volume_id}", tags(Watches))]
async fn get_watches(
    web::Path(volume_id): web::Path<VolumeId>,
) -> Result<Json<Vec<RestWatch>>, RestError> {
    let watches = GetWatches {
        resource: WatchResource::Volume(volume_id),
    }
    .request()
    .await?;
    let watches = watches.0.iter();
    let watches = watches
        .filter_map(|w| RestWatch::try_from(w).ok())
        .collect();
    Ok(Json(watches))
}

#[delete("/watches/volume/{volume_id}", tags(Watches))]
async fn del_watch(
    web::Path(volume_id): web::Path<VolumeId>,
    web::Query(watch): web::Query<WatchTypeQueryParam>,
) -> Result<JsonUnit, RestError> {
    DeleteWatch {
        resource: WatchResource::Volume(volume_id),
        callback: WatchCallback::Uri(watch.callback.to_string()),
        watch_type: WatchType::Actual,
    }
    .request()
    .await?;

    Ok(JsonUnit::default())
}
