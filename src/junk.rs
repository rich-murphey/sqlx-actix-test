use actix_web::error::ErrorInternalServerError;
use actix_web::*;
use serde::*;
use sqlx::postgres::*;
use sqlx_actix_streaming::*;

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
) -> Result<web::Json<Vec<JunkRec>>, actix_web::Error> {
    Ok(web::Json(
        sqlx::query_as::<Postgres, JunkRec>("SELECT * FROM junk OFFSET $1 LIMIT $2;")
            .bind(params.offset)
            .bind(params.limit)
            .fetch_all(&**pool)
            .await
            .map_err(ErrorInternalServerError)?,
    ))
}
#[post("/junk2")]
async fn junk2(
    web::Json(params): web::Json<JunkParams>,
    pool: web::Data<PgPool>,
) -> Result<web::Json<Vec<JunkRec>>, actix_web::Error> {
    let mut conn = pool.acquire().await.map_err(ErrorInternalServerError)?;
    Ok(web::Json(
        sqlx::query_as::<Postgres, JunkRec>("SELECT * FROM junk OFFSET $1 LIMIT $2;")
            .bind(params.offset)
            .bind(params.limit)
            .fetch_all(&mut conn)
            .await
            .map_err(ErrorInternalServerError)?,
    ))
}
//________________________________________________________________ Streaming response

#[get("/junkstream/{limit}/{offset}")]
pub async fn junkstream(path: web::Path<(i64, i64)>, pool: web::Data<PgPool>) -> Result<HttpResponse, actix_web::Error> {
    let conn = pool
        .acquire()
        .await
        .map_err(ErrorInternalServerError)?;
    Ok(HttpResponse::Ok()
       .content_type("application/json")
       .streaming(ByteStream::new(
           SelfRefStream::build((conn, path.into_inner()), move |(pool, (limit, offset))| {
               sqlx::query_as!(
                   JunkRec,
                   "select * from junk offset $1 limit $2",
                   *offset,
                   *limit,
               )
                   .fetch(pool)
           }),
           |buf: &mut BytesWriter, rec| {
               serde_json::to_writer(buf, rec).map_err(ErrorInternalServerError)
           },
       )))
}

#[get("/junkstream2/{limit}/{offset}")]
pub async fn junkstream2(
    path: web::Path<(i64, i64)>,
    pool: web::Data<PgPool>,
) -> Result<HttpResponse, actix_web::Error> {
    Ok(HttpResponse::Ok()
        .content_type("application/json")
        .streaming(ByteStream::new(
            RowStream::build(&**pool, path.into_inner(), move |conn, (limit, offset)| {
                sqlx::query_as!(
                    JunkRec,
                    "select * from junk offset $1 limit $2",
                    offset,
                    limit,
                )
                .fetch(conn)
            })
            .await
            .map_err(ErrorInternalServerError)?,
            |buf: &mut BytesWriter, rec| {
                serde_json::to_writer(buf, rec).map_err(ErrorInternalServerError)
            },
        )))
}

#[get("/junkstream3/{limit}/{offset}")]
pub async fn junkstream3(
    path: web::Path<(i64, i64)>,
    pool: web::Data<PgPool>,
) -> Result<HttpResponse, actix_web::Error> {
    let conn = pool
        .acquire()
        .await
        .map_err(ErrorInternalServerError)?;
    Ok(HttpResponse::Ok()
        .content_type("application/json")
        .streaming(ByteStream::new(
            SelfRefStream::build((conn, path.into_inner()), move |(conn, (limit, offset))| {
                sqlx::query_as!(
                    JunkRec,
                    "select * from junk offset $1 limit $2",
                    *offset,
                    *limit,
                )
                .fetch(conn)
            }),
            |buf: &mut BytesWriter, rec| {
                serde_json::to_writer(buf, rec).map_err(ErrorInternalServerError)
            },
        )))
}
