use std::{
    fs::{create_dir, write},
    path::PathBuf,
};
const INDEX: &str = r#"
<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>Simple Demo</title>
</head>
<body>
  <h1>Welcome to simple. Take a look at <a href="https://github.com/Tnixc/simple">the github</a> to get started</h1>
</body>
</html>
"#;

pub fn new(args: Vec<String>) -> std::io::Result<()> {
    let path = PathBuf::from(&args[2]);
    create_dir(&path)?;

    let src = &path.join("src");
    create_dir(src)?;
    create_dir(src.join("components"))?;
    create_dir(src.join("templates"))?;
    create_dir(src.join("data"))?;
    create_dir(src.join("public"))?;
    create_dir(src.join("pages"))?;
    write(
        src.join("pages").join("index").with_extension("html"),
        INDEX.as_bytes(),
    )?;
    println!(
        "{}",
        format!("Done. run `simple build {} to get started", args[2])
    );

    Ok(())
}
