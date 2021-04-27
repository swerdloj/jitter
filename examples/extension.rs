// Only the extension module is needed
use jitter::extension::*;

// Simple extension that appends the inputs as i32 fields
#[no_mangle]
fn transform_top_level(mut item: Item, inputs: Vec<&str>) -> ExtensionResult {
    match &mut item {
        Item::Struct(s) => {
            for param in inputs {
                s.fields.push(
                    StructField {
                        name: param.to_owned(),
                        ty: Type::i32,
                        is_public: true,
                    }.nodify()
                );
            }
        }

        _ => return Err("Not a struct".into())
    }

    Ok(vec![item])
}

// #[no_mangle]
// fn transform_statement(item: Statement) -> Vec<Statement>{

// }