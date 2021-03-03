use futures::{
    channel::mpsc::{UnboundedReceiver, UnboundedSender},
    executor, try_join,
};
use rumpsteak::{
    channel::Bidirectional, try_session, Branch, Choice, End, Message, Role, Roles, Send,
};
use std::{error::Error, result};

type Result<T> = result::Result<T, Box<dyn Error>>;

type Channel = Bidirectional<UnboundedSender<Label>, UnboundedReceiver<Label>>;

#[derive(Roles)]
struct Roles(S, R);

#[derive(Role)]
#[message(Label)]
struct S(#[route(R)] Channel);

#[derive(Role)]
#[message(Label)]
struct R(#[route(S)] Channel);

#[derive(Message)]
enum Label {
    A0(A0),
    A1(A1),
    D0(D0),
    D1(D1),
}

struct A0;
struct A1;
struct D0(i32);
struct D1(i32);

type Sender = Send<R, D0, Branch<R, SenderChoice0>>;

#[derive(Choice)]
#[message(Label)]
enum SenderChoice0 {
    A0(A0, Send<R, D1, Branch<R, SenderChoice1>>),
    A1(A1, Send<R, D0, Branch<R, SenderChoice0>>),
}

#[derive(Choice)]
#[message(Label)]
enum SenderChoice1 {
    A0(A0, Send<R, D1, Branch<R, SenderChoice1>>),
    A1(A1, End),
}

type Receiver = Branch<S, ReceiverChoice0>;

#[derive(Choice)]
#[message(Label)]
enum ReceiverChoice0 {
    D0(D0, Send<S, A0, Branch<S, ReceiverChoice1>>),
    D1(D1, Send<S, A1, Branch<S, ReceiverChoice0>>),
}

#[derive(Choice)]
#[message(Label)]
enum ReceiverChoice1 {
    D0(D0, Send<S, A0, Branch<S, ReceiverChoice1>>),
    D1(D1, Send<S, A1, End>),
}

async fn sender(role: &mut S, input: (i32, i32)) -> Result<()> {
    try_session(|mut s: Sender| async {
        let mut s = loop {
            s = {
                let s = s.send(role, D0(input.0)).await?;
                match s.branch(role).await? {
                    SenderChoice0::A0(A0, s) => break s,
                    SenderChoice0::A1(A1, s) => s,
                }
            };
        };

        let s = loop {
            s = {
                let s = s.send(role, D1(input.1)).await?;
                match s.branch(role).await? {
                    SenderChoice1::A0(A0, s) => s,
                    SenderChoice1::A1(A1, s) => break s,
                }
            };
        };

        Ok(((), s))
    })
    .await
}

async fn receiver(role: &mut R) -> Result<(i32, i32)> {
    try_session(|mut s: Receiver| async {
        let (x, mut s) = loop {
            s = match s.branch(role).await? {
                ReceiverChoice0::D0(D0(x), s) => break (x, s.send(role, A0).await?),
                ReceiverChoice0::D1(D1(_), s) => s.send(role, A1).await?,
            }
        };

        let (y, s) = loop {
            s = match s.branch(role).await? {
                ReceiverChoice1::D0(D0(_), s) => s.send(role, A0).await?,
                ReceiverChoice1::D1(D1(y), s) => break (y, s.send(role, A1).await?),
            }
        };

        Ok(((x, y), s))
    })
    .await
}

fn main() {
    let Roles(mut s, mut r) = Roles::default();

    let input = (1, 2);
    println!("input = {:?}", input);

    let (_, output) =
        executor::block_on(async { try_join!(sender(&mut s, input), receiver(&mut r)).unwrap() });
    println!("output = {:?}", output);
}
