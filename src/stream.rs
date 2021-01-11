// Copyright (c) 2020 Carey Richard Murphey.
use actix_web::*;
use futures::{prelude::*, task};
#[cfg(not(feature = "tracing"))]
use log::*;
use std::pin::Pin;
#[cfg(feature = "tracing")]
use tracing::*;

pub struct Writer<'a>(pub &'a mut web::BytesMut);

impl<'a> std::io::Write for Writer<'a> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

// this trait implements the specifcs of a given query
pub trait Context<DB, Rec, Qry>
where
    DB: sqlx::Database,
    Rec: std::fmt::Debug,
{
    // this returns a sql query object.  The query may bind
    // parameters, and the stream needs to own those parametsers, so
    // they are part of the Qry object.
    fn qry(&self) -> Qry;

    // This returns a stream of records objects.  They will be
    // converted to json strings, and joined together in an object or
    // array using the prefix(), separator(), and suffix() methods.
    fn stream<'a>(
        &self,
        pool: &'a sqlx::Pool<DB>,
        qry: &'a Qry,
    ) -> stream::BoxStream<'a, Result<Rec, sqlx::Error>>;

    // This uses serde to write a object as json to the output buffer
    fn write(&self, rec: &Rec, buf: &mut web::BytesMut) -> Result<(), String>;

    // This writes a json prefix. For arrays, the prefix is '['.
    fn prefix(&self, buf: &mut web::BytesMut);

    // This writes a json separator. For arrays, the separator is ','.
    fn separator(&self, buf: &mut web::BytesMut);

    // This writes a json suffix. For arrays, the suffix is ']'.
    fn suffix(&self, buf: &mut web::BytesMut);
}

// The stream must own everything it references, so we use ouroboros
// to acheive that.  This includes any bound parameters, owned by the
// Qry object.
#[ouroboros::self_referencing]
pub struct OwnedStream<DB: sqlx::Database, Rec, Qry: 'static> {
    pub pool: Box<sqlx::Pool<DB>>,
    pub qry: Box<Qry>,
    #[borrows(pool, qry)]
    pub stream: stream::BoxStream<'this, Result<Rec, sqlx::Error>>,
}

#[derive(Debug)]
pub enum RecStreamState {
    New,
    Started,
    Finished,
    Dead,
}
use RecStreamState::*;

pub struct RecStream<
    DB: sqlx::Database,
    Rec: std::fmt::Debug,
    Ctx: Context<DB, Rec, Qry>,
    Qry: 'static,
> {
    pub state: RecStreamState,
    pub size: usize,
    pub ctx: Box<Ctx>,
    pub owned: OwnedStream<DB, Rec, Qry>,
}
impl<DB, Rec, Ctx, Qry> Stream for RecStream<DB, Rec, Ctx, Qry>
where
    DB: sqlx::Database,
    Rec: std::fmt::Debug,
    Ctx: Context<DB, Rec, Qry>,
{
    type Item = Result<web::Bytes, actix_http::Error>;
    fn poll_next(
        mut self: Pin<&mut Self>,
        cx: &mut task::Context<'_>,
    ) -> task::Poll<Option<Self::Item>> {
        match self.state {
            Finished => {
                self.state = Dead;
                return task::Poll::Ready(None);
            }
            Dead => error!("stream polled in state: {:?}", self.state),
            _ => (),
        }
        let mut buf = web::BytesMut::with_capacity(self.size * 3 / 2);
        loop {
            match self.owned.with_stream_mut(|s| s.as_mut().poll_next(cx)) {
                task::Poll::Ready(Some(Ok(rec))) => {
                    match self.state {
                        New => {
                            self.ctx.prefix(&mut buf);
                            self.state = Started;
                        }
                        Started => self.ctx.separator(&mut buf),
                        _ => error!("got Ready(Some(Ok)) while: {:?}", self.state),
                    };
                    if let Err(e) = self.ctx.write(&rec, &mut buf) {
                        error!("write failed: {} for {:?}", e, &rec);
                    }
                    if buf.len() < self.size {
                        continue;
                    } else {
                        return task::Poll::Ready(Some(Ok(web::Bytes::from(buf))));
                    }
                }
                task::Poll::Ready(Some(Err(e))) => {
                    error!("stream pool: {:?}", e);
                    continue;
                }
                task::Poll::Ready(None) => {
                    match self.state {
                        New | Started => {
                            self.state = Finished;
                            self.ctx.suffix(&mut buf);
                            return task::Poll::Ready(Some(Ok(web::Bytes::from(buf))));
                        }
                        _ => error!("got Ready(None,) while: {:?}", self.state),
                    };
                }
                task::Poll::Pending => {
                    return if buf.is_empty() {
                        task::Poll::Pending
                    } else {
                        task::Poll::Ready(Some(Ok(web::Bytes::from(buf))))
                    };
                }
            }
        }
    }
}

#[inline(always)]
pub fn build_stream<DB, Rec, Ctx, Qry>(
    pool: sqlx::Pool<DB>,
    ctx: Ctx,
) -> RecStream<DB, Rec, Ctx, Qry>
where
    DB: sqlx::Database,
    Rec: std::fmt::Debug,
    Ctx: Context<DB, Rec, Qry>,
{
    let qry = ctx.qry();
    let owned = OwnedStreamBuilder {
        pool: Box::new(pool),
        qry: Box::new(qry),
        stream_builder: |pool, qry| ctx.stream(pool, qry),
    }
    .build();

    RecStream {
        state: New,
        size: 2048,
        ctx: Box::new(ctx),
        owned,
    }
}

#[inline(always)]
pub fn stream_response<DB, Rec: 'static, Ctx: 'static, Qry: 'static>(
    pool: sqlx::Pool<DB>,
    ctx: Ctx,
) -> Result<HttpResponse, actix_web::Error>
where
    DB: sqlx::Database,
    Rec: std::fmt::Debug,
    Ctx: Context<DB, Rec, Qry>,
{
    Ok(HttpResponse::Ok()
        .content_type("application/json")
        .streaming(build_stream(pool, ctx)))
}
