use ferrite_session::{either::*, prelude::*};

struct Ready;
struct Value(i32);

type Source = ReceiveChannel<Sink, SendValue<Vec<i32>, End>>;

type Sink =
    Rec<SendValue<Ready, ExternalChoice<Either<ReceiveValue<Value, Z>, SendValue<Vec<i32>, End>>>>>;

// Advanced Optimization: use a PartialSession directly with a single Sink channel
// in the linear context. This way when the recursion point is called, there is
// no need to reconstruct the linear context but instead use it directly.
fn source_inner(
    values: Vec<i32>,
    index: usize,
) -> PartialSession<HList![Sink], SendValue<Vec<i32>, End>> {
    let c = Z;

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
                        step(async move { source_inner(values, index + 1) })
                    )
                );
            } else {
                choose!(c, Right, forward(c))
            }
        }),
    )
}

fn source(values: Vec<i32>) -> Session<Source> {
    receive_channel(|_| source_inner(values, 0))
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
