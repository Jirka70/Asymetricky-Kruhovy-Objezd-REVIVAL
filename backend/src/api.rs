use crate::{db::DbPool, models::*, schema::*};
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug)]
pub struct ApiError(StatusCode, &'static str);
impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.0, Json(ErrorBody { error: self.1 })).into_response()
    }
}
#[derive(Serialize)]
struct ErrorBody {
    error: &'static str,
}

async fn query_db<T, F>(pool: DbPool, operation: F) -> Result<T, ApiError>
where
    T: Send + 'static,
    F: FnOnce(&mut PgConnection) -> QueryResult<T> + Send + 'static,
{
    tokio::task::spawn_blocking(move || {
        let mut connection = pool
            .get()
            .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "Database unavailable"))?;
        operation(&mut connection).map_err(|error| match error {
            diesel::result::Error::NotFound => ApiError(StatusCode::NOT_FOUND, "Record not found"),
            _ => {
                tracing::error!(%error, "Database query failed");
                ApiError(StatusCode::INTERNAL_SERVER_ERROR, "Database query failed")
            }
        })
    })
    .await
    .map_err(|_| ApiError(StatusCode::INTERNAL_SERVER_ERROR, "Database task failed"))?
}

#[derive(Default, Deserialize)]
pub struct Pagination {
    limit: Option<i64>,
    offset: Option<i64>,
}
impl Pagination {
    fn bounds(&self) -> Result<(i64, i64), ApiError> {
        let limit = self.limit.unwrap_or(100);
        let offset = self.offset.unwrap_or(0);
        if !(1..=1000).contains(&limit) || offset < 0 {
            return Err(ApiError(
                StatusCode::BAD_REQUEST,
                "limit must be 1..1000 and offset must be nonnegative",
            ));
        }
        Ok((limit, offset))
    }
}

macro_rules! list_handler {
    ($function:ident, $table:ident, $model:ident, $order:expr) => {
        async fn $function(
            State(pool): State<DbPool>,
            Query(page): Query<Pagination>,
        ) -> Result<Json<Vec<$model>>, ApiError> {
            let (limit, offset) = page.bounds()?;
            query_db(pool, move |connection| {
                $table::table
                    .order($order)
                    .limit(limit)
                    .offset(offset)
                    .select($model::as_select())
                    .load(connection)
            })
            .await
            .map(Json)
        }
    };
}
list_handler!(list_skoly, stredni_skoly, Skola, stredni_skoly::redizo);
list_handler!(list_zsj, zsj, Zsj, zsj::kod);
list_handler!(list_obory, obory, Obor, obory::kod);
list_handler!(list_nabidky, nabidka_oboru, NabidkaOboru, nabidka_oboru::id);
list_handler!(
    list_zamestnavatele,
    zamestnavatele,
    Zamestnavatel,
    zamestnavatele::id
);
list_handler!(
    list_profesni_skupiny,
    profesni_skupiny,
    ProfesniSkupina,
    profesni_skupiny::cz_isco3
);
list_handler!(
    list_poptavka,
    poptavka_profesi,
    PoptavkaProfesi,
    (
        poptavka_profesi::zamestnavatel_id,
        poptavka_profesi::cz_isco3,
        poptavka_profesi::min_vzdelani
    )
);
list_handler!(
    list_obor_profese,
    obor_profese,
    OborProfese,
    (obor_profese::cz_isco3, obor_profese::kod_oboru)
);

async fn skola(
    State(pool): State<DbPool>,
    Path(redizo): Path<String>,
) -> Result<Json<Skola>, ApiError> {
    query_db(pool, move |connection| {
        stredni_skoly::table
            .find(redizo)
            .select(Skola::as_select())
            .first(connection)
    })
    .await
    .map(Json)
}
async fn zsj_detail(
    State(pool): State<DbPool>,
    Path(kod): Path<String>,
) -> Result<Json<Zsj>, ApiError> {
    query_db(pool, move |connection| {
        zsj::table
            .find(kod)
            .select(Zsj::as_select())
            .first(connection)
    })
    .await
    .map(Json)
}
async fn health(State(pool): State<DbPool>) -> Result<Json<Health>, ApiError> {
    query_db(pool, |connection| {
        diesel::select(diesel::dsl::sql::<diesel::sql_types::Integer>("1"))
            .get_result::<i32>(connection)
    })
    .await?;
    Ok(Json(Health { status: "ok" }))
}
#[derive(Serialize)]
struct Health {
    status: &'static str,
}

pub fn router(pool: DbPool) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/api/skoly", get(list_skoly))
        .route("/api/skoly/{redizo}", get(skola))
        .route("/api/zsj", get(list_zsj))
        .route("/api/zsj/{kod}", get(zsj_detail))
        .route("/api/obory", get(list_obory))
        .route("/api/nabidky", get(list_nabidky))
        .route("/api/zamestnavatele", get(list_zamestnavatele))
        .route("/api/profesni-skupiny", get(list_profesni_skupiny))
        .route("/api/poptavka-profesi", get(list_poptavka))
        .route("/api/obor-profese", get(list_obor_profese))
        .with_state(pool)
        .merge(crate::contract::router())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pagination_limits() {
        assert_eq!(Pagination::default().bounds().unwrap(), (100, 0));
        for limit in [0, -1, 1001] {
            assert!(
                Pagination {
                    limit: Some(limit),
                    offset: None
                }
                .bounds()
                .is_err()
            );
        }
        assert!(
            Pagination {
                limit: Some(1),
                offset: Some(-1)
            }
            .bounds()
            .is_err()
        );
        assert_eq!(
            Pagination {
                limit: Some(1000),
                offset: Some(5)
            }
            .bounds()
            .unwrap(),
            (1000, 5)
        );
    }
}
