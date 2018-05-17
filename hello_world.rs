#![feature(try_from)]

extern crate futures;
extern crate ruma_client;
extern crate ruma_events;
extern crate ruma_identifiers;
extern crate tokio_core;
extern crate url;

use std::convert::TryFrom;
use std::env;
use std::process::exit;

use futures::Future;
use ruma_client::Client;
use ruma_client::api::r0;
use ruma_events::EventType;
use ruma_events::room::message::{MessageEventContent, MessageType, TextMessageEventContent};
use ruma_identifiers::RoomAliasId;
use tokio_core::reactor::{Core, Handle};
use url::Url;

// from https://stackoverflow.com/a/43992218/1592377
#[macro_export]
macro_rules! clone {
    (@param _) => ( _ );
    (@param $x:ident) => ( $x );
    ($($n:ident),+ => move || $body:expr) => (
        {
            $( let $n = $n.clone(); )+
            move || $body
        }
    );
    ($($n:ident),+ => move |$($p:tt),+| $body:expr) => (
        {
            $( let $n = $n.clone(); )+
            move |$(clone!(@param $p),)+| $body
        }
    );
}

fn hello_world(
    tokio_handle: &Handle,
    homeserver_url: Url,
    room: String,
) -> impl Future<Item = (), Error = ruma_client::Error> + 'static {
    let client = Client::https(tokio_handle, homeserver_url, None).unwrap();

    client.register_guest().and_then(clone!(client => move |_| {
        r0::alias::get_alias::call(client, r0::alias::get_alias::Request {
            room_alias: RoomAliasId::try_from(&room[..]).unwrap(),
        })
    })).and_then(clone!(client => move |response| {
        let room_id = response.room_id;

        r0::membership::join_room_by_id::call(client.clone(), r0::membership::join_room_by_id::Request {
            room_id: room_id.clone(),
            third_party_signed: None,
        }).and_then(move |_| {
            r0::send::send_message_event::call(client, r0::send::send_message_event::Request {
                room_id: room_id,
                event_type: EventType::RoomMessage,
                txn_id: "1".to_owned(),
                data: MessageEventContent::Text(TextMessageEventContent {
                    body: "Hello World!".to_owned(),
                    msgtype: MessageType::Text,
                }),
            })
        })
    })).map(|_| ())
}

fn main() {
    let (homeserver_url, room) = match (env::args().nth(1), env::args().nth(2)) {
        (Some(a), Some(b)) => (a, b),
        _ => {
            eprintln!("Usage: {} <homeserver_url> <room>", env::args().next().unwrap());
            exit(1)
        }
    };

    let mut core = Core::new().unwrap();
    let handle = core.handle();
    let server = Url::parse(&homeserver_url).unwrap();

    core.run(hello_world(&handle, server, room)).unwrap();
}
