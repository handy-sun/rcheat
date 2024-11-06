use mlua::{Lua, ObjectLike, Table, Value};
use std::path::PathBuf;
use std::{env, fs};
use walkdir::WalkDir;

fn print_table(tab: &Table, indent: usize) -> mlua::Result<String> {
    let prefix = vec!["  "; indent].concat();
    let mut output: Vec<_> = Vec::with_capacity(16);

    for pair in tab.pairs::<Value, Value>() {
        let (key, value) = pair?;
        let key_str = match key {
            Value::Integer(i) => i.to_string(),
            Value::String(s) => s.to_string_lossy().to_string(),
            _ => String::from("nil"),
        };

        match value {
            Value::Table(child) => {
                output.push(format!("{}{}:\n", prefix, key_str));
                output.push(print_table(&child, indent + 1)?);
            }
            Value::Integer(integer) => output.push(format!("{}{}: {}\n", prefix, key_str, integer)),
            Value::String(s) => output.push(format!("{}{}: {}\n", prefix, key_str, s.to_string_lossy())),
            _ => (),
        }
    }
    Ok(output.concat())
}

#[allow(dead_code)]
pub fn dump_with_lua(lua_src_path: &PathBuf, bytes: &[u8], type_name: &str) -> mlua::Result<()> {
    env::set_current_dir(&lua_src_path)?;

    // This loads the default Lua std library *without* the debug library.
    let lua = Lua::new();
    let globals = lua.globals();

    let walker = WalkDir::new(lua_src_path)
        .max_depth(1)
        .follow_links(true)
        .into_iter();
    for res_entry in walker {
        if let Ok(entry) = res_entry {
            if entry.path().is_dir() {
                continue;
            }
            println!("{:?}", entry.path());
            let file_content = fs::read_to_string(entry.path())?;
            lua.load(&file_content).exec()?;
        }
    }

    let lua_str = lua.create_string(bytes)?;
    let create_func = format!("new_{}", type_name);

    let _res: Table = globals
        .get::<Table>("Structure")?
        .call_function(create_func.as_ref(), lua_str)?;

    let inner: Table = _res.get(type_name)?;
    println!("{}", print_table(&inner, 0)?);
    Ok(())
}
