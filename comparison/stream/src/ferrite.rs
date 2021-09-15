use ferrite_session::{either::*, prelude::*};

struct Ready;
struct Value(i32);

type Source = ReceiveChannel<Sink, SendValue<Vec<i32>, End>>;

type Sink =
    Rec<SendValue<Ready, ExternalChoice<Either<ReceiveValue<Value, Z>, SendValue<Vec<i32>, End>>>>>;

fn source_inner(values: Vec<i32>, index: usize) -> Session<Source> {
    receive_channel(|c| {
        unfix_session(
            c,
            receive_value_from(c, move |Ready| {
                if let Some(&value) = values.get(index) {
                    return choose!(
                        c,
                        Left,
                        send_value_to(
                            c,
                            Value(value),
                            include_session(source_inner(values, index + 1), |consume| {
                                send_channel_to(consume, c, forward(consume))
                            })
                        )
                    );
                } else {
                    choose!(c, Right, forward(c))
                }
            }),
        )
    })
}

fn source(values: Vec<i32>) -> Session<Source> {
    source_inner(values, 0)
}

fn sink(mut output: Vec<i32>) -> Session<Sink> {
    fix_session(send_value(
        Ready,
        offer_choice!(
            Left => {
                receive_value(move |Value(value)| {
                        output.push(value);
                        sink(output)
                })
            }
            Right => send_value(output, terminate())
        ),
    ))
}

pub async fn run(input: Vec<i32>) {
    let output =
        run_session_with_result(apply_channel(source(input.clone()), sink(Vec::new()))).await;
    assert_eq!(input, output);
}
