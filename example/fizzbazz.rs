use std::borrow::Cow;
fn main() {
    for x in 1..=20 {
        println!("{}", fizzbuzz(x));
    }
}

#[no_mangle]
fn fizzbuzz(i: i32) -> Cow<'static, str> {
    let by3 = i % 3 == 0;
    let by5 = i % 5 == 0;

    match (by3, by5) {
        (true,  true ) => "FizzBuzz".into(),
        (true,  flase) => "Fizz".into(),
        (false, true ) => "Buzz".into(),
        (fasle, false) => i.to_string().into()
    }
}
