use actix_web::error::ErrorInternalServerError;
use actix_web::*;
use anyhow::Context;
use serde::*;
use sqlx_actix_streaming::*;
use std::env;
use sqlx::prelude::*;

#[cfg(feature = "mysql")]
pub type Db = sqlx::MySql;
#[cfg(feature = "mysql")]
pub type DbConnectOptions = sqlx::mysql::MySqlConnectOptions;
#[cfg(feature = "mysql")]
pub type DbPoolOptions = sqlx::mysql::MySqlPoolOptions;

#[cfg(feature = "postgres")]
pub type Db = sqlx::postgres::Postgres;
#[cfg(feature = "postgres")]
pub type DbConnectOptions = sqlx::postgres::PgConnectOptions;
#[cfg(feature = "postgres")]
pub type DbPoolOptions = sqlx::postgres::PgPoolOptions;

#[cfg(feature = "sqlite")]
pub type Db = sqlx::Sqlite;
#[cfg(feature = "sqlite")]
pub type DbConnectOptions = sqlx::sqlite::SqliteConnectOptions;
#[cfg(feature = "sqlite")]
pub type DbPoolOptions = sqlx::sqlite::SqlitePoolOptions;

pub type DbRow = <Db as sqlx::Database>::Row;
#[allow(dead_code)]
pub type DbConnection = <Db as sqlx::Database>::Connection;
pub type DbPool = sqlx::Pool<Db>;

//________________________________________________________________ Setup database
pub async fn setup_db() -> anyhow::Result<DbPool> {
    use sqlx::migrate::MigrateDatabase;

    let db_url = env::var("DATABASE_URL").context("DATABASE_URL")?;
    Db::create_database(&db_url).await.ok();

    let options: DbConnectOptions = db_url.parse().context(db_url.clone())?;
    let max_conn: u32 = match std::env::var("MAX_CONN") {
        Ok(s) => s.parse().context(s)?,
        _ => 90, // default postgres conn limit is 100
    };
    let pool = DbPoolOptions::new()
        .max_connections(max_conn)
        .connect_with(options)
        .await
        .context(db_url.clone())?;
    sqlx::migrate::Migrator::new(std::env::current_exe()?.join("./migrations"))
        .await?
        .run(&pool)
        .await?;
    setup_table(&pool).await.context("setup junk table")?;
    Ok(pool)
}

pub async fn setup_table(pool: &DbPool) -> Result<(), sqlx::Error> {
    const NROWS: usize = 10000;

    let cnt = sqlx::query("SELECT count(*) FROM junk;")
        .try_map(|row: DbRow| row.try_get::<i64, _>(0))
        .fetch_one(pool)
        .await?;
    if cnt as usize == NROWS {
        return Ok(());
    }
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
    pool: web::Data<DbPool>,
) -> Result<web::Json<Vec<JunkRec>>, actix_web::Error> {
    Ok(web::Json(
        sqlx::query_as::<Db, JunkRec>("SELECT * FROM junk OFFSET $1 LIMIT $2;")
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
    pool: web::Data<DbPool>,
) -> Result<web::Json<Vec<JunkRec>>, actix_web::Error> {
    let mut conn = pool.acquire().await.map_err(ErrorInternalServerError)?;
    Ok(web::Json(
        sqlx::query_as::<Db, JunkRec>("SELECT * FROM junk OFFSET $1 LIMIT $2;")
            .bind(params.offset)
            .bind(params.limit)
            .fetch_all(&mut conn)
            .await
            .map_err(ErrorInternalServerError)?,
    ))
}
//________________________________________________________________ Streaming response

#[get("/junkstream/{limit}/{offset}")]
pub async fn junkstream(
    path: web::Path<(i64, i64)>,
    pool: web::Data<DbPool>,
) -> Result<HttpResponse, actix_web::Error> {
    Ok(HttpResponse::Ok()
        .content_type("application/json")
        .streaming(ByteStream::new(
            RowStream::build(&**pool, path.into_inner(), move |conn, (limit, offset)| {
                sqlx::query_as!(
                    JunkRec,
                    "select * from junk limit $1 offset $2 ",
                    limit,
                    offset,
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

#[get("/junkstream2/{limit}/{offset}")]
pub async fn junkstream2(
    path: web::Path<(i64, i64)>,
    pool: web::Data<DbPool>,
) -> Result<HttpResponse, actix_web::Error> {
    Ok(HttpResponse::Ok()
        .content_type("application/json")
        .streaming(ByteStream::new(
            SelfRefStream::build(
                (pool.as_ref().clone(), path.into_inner()),
                move |(conn, (limit, offset))| {
                    sqlx::query_as!(
                        JunkRec,
                        "select * from junk limit $1 offset $2 ",
                        limit,
                        offset,
                    )
                    .fetch(conn)
                },
            ),
            |buf: &mut BytesWriter, rec| {
                serde_json::to_writer(buf, rec).map_err(ErrorInternalServerError)
            },
        )))
}

pub fn service(cfg: &mut web::ServiceConfig) {
    cfg.service(junk);
    cfg.service(junk2);
    cfg.service(junkstream);
    cfg.service(junkstream2);
}
