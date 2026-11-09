// Puts the Spark icon and name inside spark.exe (what you see in Explorer).
fn main() {
    println!("cargo:rerun-if-changed=assets/spark.ico");
    #[cfg(windows)]
    {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("assets/spark.ico");
        res.set("ProductName", "Spark Game Maker");
        res.set("FileDescription", "Spark Game Maker - make your own video games");
        if let Err(e) = res.compile() {
            println!("cargo:warning=could not put the icon in the program: {e}");
        }
    }
}
