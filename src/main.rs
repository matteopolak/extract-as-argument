mod private {
	pub struct WithParts;
	pub struct WithRequest;
}

#[derive(Clone, Copy)]
struct RequestParts {
	count: u8,
}

#[derive(Clone)]
struct Request {
	parts: RequestParts,
	expensive: Vec<u8>,
}

struct Response {
	content: String,
}

trait FromRequestParts<S> {
	fn from_request_parts(parts: &mut RequestParts, state: S) -> Self;
}

trait FromRequest<S, X = private::WithRequest> {
	fn from_request(req: Request, state: S) -> Self;
}

trait Handler<T, S> {
	fn call(self, req: Request, state: S) -> Response;
}

impl<T, S> FromRequest<S, private::WithParts> for T
where
	T: FromRequestParts<S>,
{
	fn from_request(mut req: Request, state: S) -> Self {
		T::from_request_parts(&mut req.parts, state)
	}
}

impl<S> FromRequestParts<S> for () {
	fn from_request_parts(_: &mut RequestParts, _: S) -> Self {}
}

impl<S, F> Handler<(), S> for F
where
	F: Fn() -> Response,
{
	fn call(self, _: Request, _: S) -> Response {
		self()
	}
}

impl<S, F, M, T1> Handler<(M, T1), S> for F
where
	F: FnOnce(T1) -> Response,
	T1: FromRequest<S, M>,
{
	fn call(self, req: Request, state: S) -> Response {
		let t1 = T1::from_request(req, state);
		self(t1)
	}
}

impl<S, F, M, T1, T2> Handler<(M, T1, T2), S> for F
where
	F: FnOnce(T1, T2) -> Response,
	S: Clone,
	T1: FromRequestParts<S>,
	T2: FromRequest<S, M>,
{
	fn call(self, mut req: Request, state: S) -> Response {
		let t1 = T1::from_request_parts(&mut req.parts, state.clone());
		let t2 = T2::from_request(req, state);

		self(t1, t2)
	}
}

struct State<S>(S);

impl<S> FromRequestParts<S> for State<S> {
	fn from_request_parts(_: &mut RequestParts, state: S) -> Self {
		Self(state)
	}
}

struct Count(u8);

impl<S> FromRequestParts<S> for Count {
	fn from_request_parts(parts: &mut RequestParts, _: S) -> Self {
		Self(parts.count)
	}
}

struct Expensive(Vec<u8>);

impl<S> FromRequest<S> for Expensive {
	fn from_request(req: Request, _: S) -> Self {
		Self(req.expensive)
	}
}

struct Json<T>(T);

impl<S, T> FromRequest<S> for Json<T>
where
	T: serde::de::DeserializeOwned,
{
	fn from_request(req: Request, _: S) -> Self {
		Self(serde_json::from_slice(&req.expensive).expect("expected valid json"))
	}
}

fn simple() -> Response {
	Response {
		content: "Hello, world!".to_string(),
	}
}

fn with_count_and_state(State(state): State<u8>, Count(count): Count) -> Response {
	Response {
		content: format!("state: {state}, count: {count}"),
	}
}

fn with_state_and_expensive(State(state): State<u8>, Expensive(expensive): Expensive) -> Response {
	Response {
		content: format!("state: {state}, expensive: {}", expensive.len()),
	}
}

#[derive(serde::Deserialize)]
struct Body {
	repeat: usize,
	text: String,
}

fn with_json(Json(body): Json<Body>) -> Response {
	Response {
		content: body.text.repeat(body.repeat),
	}
}

fn get<S, H, T>(handler: H) -> impl Fn(Request, S) -> Response
where
	H: Handler<T, S> + Copy,
{
	move |req, state| handler.call(req, state)
}

fn main() {
	let state = 42;
	let request = Request {
		parts: RequestParts { count: 10 },
		expensive: br#"{
			"repeat": 6,
			"text": "hi"
		}"#
		.to_vec(),
	};

	let route = get(simple);
	let response = route(request.clone(), state);

	assert_eq!(response.content, "Hello, world!");

	let route = get(with_count_and_state);
	let response = route(request.clone(), state);

	assert_eq!(response.content, "state: 42, count: 10");

	let route = get(with_state_and_expensive);
	let response = route(request.clone(), state);

	assert_eq!(response.content, "state: 42, expensive: 37");

	let route = get(with_json);
	let response = route(request.clone(), state);

	assert_eq!(response.content, "hihihihihihi");
}
