use super::*;
use actix_web::web::Path;
use common_lib::types::v0::message_bus::{DestroyNexus, Filter, ShareNexus, UnshareNexus};
use mbus_api::{
    message_bus::v0::{BusError, MessageBus, MessageBusTrait},
    ReplyErrorKind, ResourceKind,
};

async fn destroy_nexus(filter: Filter) -> Result<(), RestError<RestJsonError>> {
    let destroy = match filter.clone() {
        Filter::NodeNexus(node_id, nexus_id) => DestroyNexus {
            node: node_id,
            uuid: nexus_id,
        },
        Filter::Nexus(nexus_id) => {
            let node_id = match MessageBus::get_nexus(filter).await {
                Ok(nexus) => nexus.node,
                Err(error) => return Err(RestError::from(error)),
            };
            DestroyNexus {
                node: node_id,
                uuid: nexus_id,
            }
        }
        _ => {
            return Err(RestError::from(BusError {
                kind: ReplyErrorKind::Internal,
                resource: ResourceKind::Nexus,
                source: "destroy_nexus".to_string(),
                extra: "invalid filter for resource".to_string(),
            }))
        }
    };

    MessageBus::destroy_nexus(destroy).await?;
    Ok(())
}

#[async_trait::async_trait]
impl apis::NexusesApi for RestApi {
    async fn del_nexus(Path(nexus_id): Path<String>) -> Result<(), RestError<RestJsonError>> {
        destroy_nexus(Filter::Nexus(nexus_id.into())).await
    }

    async fn del_node_nexus(
        Path((node_id, nexus_id)): Path<(String, String)>,
    ) -> Result<(), RestError<RestJsonError>> {
        destroy_nexus(Filter::NodeNexus(node_id.into(), nexus_id.into())).await
    }

    async fn del_node_nexus_share(
        Path((node_id, nexus_id)): Path<(String, String)>,
    ) -> Result<(), RestError<RestJsonError>> {
        MessageBus::unshare_nexus(UnshareNexus {
            node: node_id.into(),
            uuid: nexus_id.into(),
        })
        .await?;
        Ok(())
    }

    async fn get_nexus(
        Path(nexus_id): Path<String>,
    ) -> Result<Json<models::Nexus>, RestError<RestJsonError>> {
        let nexus = MessageBus::get_nexus(Filter::Nexus(nexus_id.into())).await?;
        Ok(Json(nexus.into()))
    }

    async fn get_nexuses() -> Result<Json<Vec<models::Nexus>>, RestError<RestJsonError>> {
        let nexuses = MessageBus::get_nexuses(Filter::None).await?;
        Ok(Json(nexuses.into_iter().map(From::from).collect()))
    }

    async fn get_node_nexus(
        Path((node_id, nexus_id)): Path<(String, String)>,
    ) -> Result<Json<models::Nexus>, RestError<RestJsonError>> {
        let nexus =
            MessageBus::get_nexus(Filter::NodeNexus(node_id.into(), nexus_id.into())).await?;
        Ok(Json(nexus.into()))
    }

    async fn get_node_nexuses(
        Path(id): Path<String>,
    ) -> Result<Json<Vec<models::Nexus>>, RestError<RestJsonError>> {
        let nexuses = MessageBus::get_nexuses(Filter::Node(id.into())).await?;
        Ok(Json(nexuses.into_iter().map(From::from).collect()))
    }

    async fn put_node_nexus(
        Path((node_id, nexus_id)): Path<(String, String)>,
        Json(create_nexus_body): Json<models::CreateNexusBody>,
    ) -> Result<Json<models::Nexus>, RestError<RestJsonError>> {
        let create =
            CreateNexusBody::from(create_nexus_body).bus_request(node_id.into(), nexus_id.into());
        let nexus = MessageBus::create_nexus(create).await?;
        Ok(Json(nexus.into()))
    }

    async fn put_node_nexus_share(
        Path((node_id, nexus_id, protocol)): Path<(String, String, models::NexusShareProtocol)>,
    ) -> Result<Json<String>, RestError<RestJsonError>> {
        let share = ShareNexus {
            node: node_id.into(),
            uuid: nexus_id.into(),
            key: None,
            protocol: protocol.into(),
        };
        let share_uri = MessageBus::share_nexus(share).await?;
        Ok(Json(share_uri))
    }
}
