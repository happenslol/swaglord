use std::fs;
use tera::Tera;
use crate::{Error, gen::TemplateContext};

pub fn load_templates(path: &str) -> Result<Tera, Error> {
    let path = format!("templates/{}/*.tera", path);
    let result = Tera::new(&path)?;
    Ok(result)
}

pub fn write_template<T>(tera: &Tera, values: &Vec<T>) -> Result<(), Error>
where
    T: TemplateContext
{
    for it in values {
        let path = format!("./out/{}", it.filename());
        let rendered = tera.render_value(it.template(), it)?;
        fs::write(&path, &rendered)?;
    }

    Ok(())
}
