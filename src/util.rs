use std::fs;
use tera::Tera;
use serde_json;
use crate::{Error, gen::TemplateContext};

pub fn load_templates(path: &str) -> Result<Tera, Error> {
    let path = format!("templates/{}/*.tera", path);
    let result = Tera::new(&path)?;
    Ok(result)
}

pub fn write_templates<T>(tera: &Tera, values: &Vec<T>, subdir: Option<&str>) -> Result<(), Error>
where
    T: TemplateContext
{
    let dest = subdir.map_or(
        String::from("./out"),
        |it| format!("./out/{}", it),
    );

    fs::create_dir_all(&dest).unwrap();
    for it in values {
        println!("writing {}", it.filename());

        let path = format!("{}/{}", dest, it.filename());
        let rendered = tera.render_value(it.template(), it)?;
        fs::write(&path, &rendered)?;
    }

    Ok(())
}
