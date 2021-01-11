use super::error::ApiError;
use crate::stream::*;
use actix_web::*;
use chrono::prelude::*;
use futures::prelude::*;
use serde::*;
use sqlx::postgres::*;
use sqlx::prelude::*;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct FilmRec {
    pub film_id: i32,
    pub title: String,
    pub description: String,
    pub release_year: i32,
    pub language_id: i16,
    pub original_language_id: Option<i32>,
    pub rental_duration: i16,
    // pub rental_rate: Decimal,
    pub length: i16,
    // pub replacement_cost: Decimal,
    // pub rating: String,
    pub last_update: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilmParams {
    pub offset: i64,
    pub limit: i64,
}
#[derive(Debug)]
struct FilmCtx {
    pub params: FilmParams,
}
struct FilmQry {
    pub sql: String,
}
impl Context<Postgres, FilmRec, FilmQry> for FilmCtx {
    fn qry(&self) -> FilmQry {
        let sql = "
SELECT * FROM film
OFFSET $1 LIMIT $2"
            .to_string();
        FilmQry { sql }
    }
    fn stream<'a>(
        &self,
        pool: &'a PgPool,
        qry: &'a FilmQry,
    ) -> stream::BoxStream<'a, Result<FilmRec, sqlx::Error>> {
        sqlx::query_as::<Postgres, FilmRec>(&qry.sql)
            .bind(self.params.offset)
            .bind(self.params.limit)
            .fetch(pool)
    }
    #[inline(always)]
    fn write<'a>(&self, rec: &'a FilmRec, buf: &'a mut web::BytesMut) -> Result<(), String> {
        serde_json::to_writer(Writer(buf), &rec).map_err(|e| e.to_string())
    }
    #[inline(always)]
    fn prefix(&self, buf: &mut web::BytesMut) {
        buf.extend_from_slice(b"[");
    }
    #[inline(always)]
    fn separator(&self, buf: &mut web::BytesMut) {
        buf.extend_from_slice(b",");
    }
    #[inline(always)]
    fn suffix(&self, buf: &mut web::BytesMut) {
        buf.extend_from_slice(b"]");
    }
}

#[post("/films")]
async fn films(
    web::Json(params): web::Json<FilmParams>,
    pool: web::Data<PgPool>,
) -> Result<web::Json<Vec<FilmRec>>, ApiError> {
    Ok(web::Json(
        sqlx::query_as::<Postgres, FilmRec>("SELECT * FROM film OFFSET $1 LIMIT $2;")
            .bind(params.offset)
            .bind(params.limit)
            .fetch_all(&**pool)
            .await?,
    ))
}

#[post("/filmstream")]
pub async fn filmstream(
    web::Json(params): web::Json<FilmParams>,
    pool: web::Data<PgPool>,
) -> Result<HttpResponse, actix_web::Error> {
    stream_response(pool.into_inner().as_ref().clone(), FilmCtx { params })
}
