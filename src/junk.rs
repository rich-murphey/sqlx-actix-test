use super::error::ApiError;
use crate::stream::*;
use actix_web::*;
use futures::prelude::*;
use serde::*;
use sqlx::postgres::*;

//________________________________________________________________ Setup database
const NROWS: usize = 10000;

pub async fn setup_db(pool: &PgPool) -> Result<(), sqlx::Error> {
    if let Err(_e) = sqlx::query("SELECT count(*) FROM junk;")
        .execute(pool)
        .await
    {
        sqlx::query("DROP TABLE IF EXISTS junk;")
            .execute(pool)
            .await?;
        sqlx::query(
            "
CREATE TABLE junk (
    id      BIGSERIAL PRIMARY KEY,
    jsn     Jsonb NOT NULL
);",
        )
        .execute(pool)
        .await?;
        let jsn = serde_json::json!(
            {
                "firstName": "John",
                "lastName": "Doe",
                "gender": "man",
                "age": 24,
                "address": {
                    "streetAddress": "126",
                    "city": "San Jone",
                    "state": "CA",
                    "postalCode": "394221"
                },
                "phoneNumbers": [
                    { "type": "home", "number": "7383627627" }
                ],
                "colors": [
                    { "color": "red", "value": "#f00" },
                    { "color": "green", "value": "#0f0" },
                    { "color": "blue", "value": "#00f" },
                    { "color": "cyan", "value": "#0ff" },
                    { "color": "magenta", "value": "#f0f" },
                    { "color": "yellow", "value": "#ff0" },
                    { "color": "black", "value": "#000" }
                ],
                "batters": {
                    "batter":
                    [
                        { "id": "1001", "type": "Regular" },
                        { "id": "1002", "type": "Chocolate" },
                        { "id": "1003", "type": "Blueberry" },
                        { "id": "1004", "type": "Devil's Food" }
                    ]
                },
                "topping": [
                    { "id": "5001", "type": "None" },
                    { "id": "5002", "type": "Glazed" },
                    { "id": "5005", "type": "Sugar" },
                    { "id": "5007", "type": "Powdered Sugar" },
                    { "id": "5006", "type": "Chocolate with Sprinkles" },
                    { "id": "5003", "type": "Chocolate" },
                    { "id": "5004", "type": "Maple" }
                ],
            }
        );
        let mut conn = pool.acquire().await?;
        for _i in 0..NROWS {
            sqlx::query("INSERT INTO junk ( jsn ) VALUES ( $1 )")
                .bind(&jsn)
                .execute(&mut conn)
                .await?;
        }
    }
    Ok(())
}
//________________________________________________________________ Buffered response
#[derive(Debug, Default, sqlx::FromRow, Serialize, Deserialize)]
struct JunkRec {
    pub id: i64,
    pub jsn: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JunkParams {
    pub offset: i64,
    pub limit: i64,
}

#[post("/junk")]
async fn junk(
    web::Json(params): web::Json<JunkParams>,
    pool: web::Data<PgPool>,
) -> Result<web::Json<Vec<JunkRec>>, ApiError> {
    Ok(web::Json(
        sqlx::query_as::<Postgres, JunkRec>("SELECT * FROM junk OFFSET $1 LIMIT $2;")
            .bind(params.offset)
            .bind(params.limit)
            .fetch_all(&**pool)
            .await?,
    ))
}
//________________________________________________________________ Streaming response
#[derive(Debug)]
struct JunkCtx {
    pub params: JunkParams,
}
struct JunkQry {
    pub sql: String,
}
impl Context<Postgres, JunkRec, JunkQry> for JunkCtx {
    fn qry(&self) -> JunkQry {
        let sql = "
SELECT * FROM junk
OFFSET $1 LIMIT $2"
            .to_string();
        JunkQry { sql }
    }
    fn stream<'a>(
        &self,
        pool: &'a PgPool,
        qry: &'a JunkQry,
    ) -> stream::BoxStream<'a, Result<JunkRec, sqlx::Error>> {
        sqlx::query_as::<Postgres, JunkRec>(&qry.sql)
            .bind(self.params.offset)
            .bind(self.params.limit)
            .fetch(pool)
    }
    #[inline(always)]
    fn write<'a>(&self, rec: &'a JunkRec, buf: &'a mut web::BytesMut) -> Result<(), String> {
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

#[post("/junkstream")]
pub async fn junkstream<'a>(
    web::Json(params): web::Json<JunkParams>,
    pool: web::Data<PgPool>,
) -> Result<HttpResponse, actix_web::Error> {
    stream_response(pool.into_inner().as_ref().clone(), JunkCtx { params })
}
