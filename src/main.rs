use std::io::{stdin, stdout, Read, Write};
use aws_config::BehaviorVersion;
use comrak::{format_commonmark, parse_document, Arena, Options};
use comrak::nodes::{AstNode, NodeValue};
use bpaf::*;
use bpaf_derive::Bpaf;

async fn translate_md_ast<'a>(ast: &'a AstNode<'a>, from_lang: &str, to_lang: &str) {
    let config = aws_config::load_defaults(BehaviorVersion::latest()).await;
    let translate = aws_sdk_translate::Client::new(&config);
    for node in ast.descendants() {
        let mut node_value = node.data.borrow_mut();
        if let NodeValue::Text(ref mut text) = node_value.value {
            let result = translate.translate_text()
                .set_target_language_code(Some(to_lang.to_string()))
                .set_source_language_code(Some(from_lang.to_string()))
                .text(text.clone()).send().await.unwrap();
            *text = result.translated_text;
        } else if let NodeValue::CodeBlock(ref mut code) = node_value.value {
            let result = translate.translate_text()
                .set_target_language_code(Some(to_lang.to_string()))
                .set_source_language_code(Some(from_lang.to_string()))
                .text(code.literal.clone()).send().await.unwrap();
            code.literal = result.translated_text;
        } else if let NodeValue::Code(ref mut code) = node_value.value {
            let result = translate.translate_text()
                .set_target_language_code(Some(to_lang.to_string()))
                .set_source_language_code(Some(from_lang.to_string()))
                .text(code.literal.clone()).send().await.unwrap();
            code.literal = result.translated_text;
        }
    }
}

fn stringify_md<'a>(ast: &'a AstNode<'a>) -> String {
    let mut buf = vec![];
    let mut options = Options::default();
    options.render.prefer_fenced = true;

    format_commonmark(ast, &options, &mut buf).unwrap();

    return String::from_utf8(buf).unwrap();
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

    let from_lang = if let Some(val) = args.from_lang { val } else { "ja".to_string() };
    let to_lang = if let Some(val) = args.to_lang { val } else { "en".to_string() };

    let mut data = vec![];
    stdin().read_to_end(&mut data).unwrap();

    let md_text = String::from_utf8(data).unwrap();

    let text = translate_md(&md_text, &from_lang, &to_lang).await.to_string();

    stdout().write_all(text.as_bytes()).unwrap();

    return Ok(());
}
