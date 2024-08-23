use std::io::{stdin, stdout, Read, Write};
use aws_config::BehaviorVersion;
use comrak::{format_commonmark, parse_document, Arena, Options};
use comrak::nodes::{AstNode, NodeValue};
use lambda_http::{run, service_fn, tracing, Body, Error, Request, RequestExt, Response};
use bpaf::*;
use std::path::PathBuf;
use bpaf_derive::Bpaf;

async fn translate_md_ast<'a>(ast: &'a AstNode<'a>, from_lang: &str, to_lang: &str) {
    let config = aws_config::load_defaults(BehaviorVersion::latest()).await;
    let translate = aws_sdk_translate::Client::new(&config);
    for node in ast.descendants() {

        if let NodeValue::Text(ref mut text) = node.data.borrow_mut().value {
            let result = translate.translate_text()
                .set_target_language_code(Some(to_lang.to_string()))
                .set_source_language_code(Some(from_lang.to_string()))
                .text(text.clone()).send().await.unwrap();
            *text = result.translated_text;
        }
    }
}

fn stringify_md<'a>(ast: &'a AstNode<'a>) -> String {
    let mut buf = Vec::new();

    format_commonmark(ast, &Options::default(), &mut buf).unwrap();

    return std::str::from_utf8(buf.as_slice()).unwrap().to_string();
}

fn parse_md<'a>(arena: &'a Arena<AstNode<'a>>, md_text: &str) -> &'a AstNode<'a> {
    let root = parse_document(
        &arena,
        md_text,
        &Options::default());

    return root;
}

async fn translate_md(md_text: &str, from_lang: &str, to_lang: &str) -> String {
    let arena = Arena::<AstNode>::new();
    let ast = parse_md(&arena, md_text);

    translate_md_ast(ast, from_lang, to_lang).await;

    let text = stringify_md(ast);

    return text;
}

/// This is the main body for the function.
/// Write your code inside it.
/// There are some code example in the following URLs:
/// - https://github.com/awslabs/aws-lambda-rust-runtime/tree/main/examples
async fn function_handler(event: Request) -> Result<Response<Body>, Error> {
    // Extract some useful information from the request
    let who = event
        .query_string_parameters_ref()
        .and_then(|params| params.first("name"))
        .unwrap_or("world");
    let message = format!("Hello {who}, this is an AWS Lambda HTTP request");

    // Return something that implements IntoResponse.
    // It will be serialized to the right response event automatically by the runtime
    let resp = Response::builder()
        .status(200)
        .header("content-type", "text/html")
        .body(message.into())
        .map_err(Box::new)?;
    Ok(resp)
}

#[allow(dead_code)]
#[derive(
    Bpaf,
    Clone,
    Debug,
)]
#[bpaf(
    options,
    version,
)]
struct Opts {
    profile: Option<String>,
    from_lang: Option<String>,
    to_lang: Option<String>
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let args = opts().run();
    //tracing::init_default_subscriber();
    //run(service_fn(function_handler)).await

    let from_lang = if let Some(val) = args.from_lang { val } else { "ja".to_string() };
    let to_lang = if let Some(val) = args.to_lang { val } else { "en".to_string() };

    let mut data = Vec::new();
    stdin().read_to_end(&mut data).unwrap();

    let md_text = std::str::from_utf8(data.as_slice()).unwrap().to_string();

    let text = translate_md(&md_text, &from_lang, &to_lang).await.to_string();

    stdout().write_all(text.as_bytes());

    return Ok(());
}
