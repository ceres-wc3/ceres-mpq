use clap::{
    app_from_crate, crate_authors, crate_description, crate_name, crate_version, AppSettings, Arg,
    ArgMatches, SubCommand,
};

fn main() {
    let matches = app_from_crate!()
        .setting(AppSettings::SubcommandRequired)
        .setting(AppSettings::DisableHelpSubcommand)
        .setting(AppSettings::GlobalVersion)
        .setting(AppSettings::ColorNever)
        .subcommand(
            SubCommand::with_name("extract")
                .about("extracts files from an archive")
                .arg(
                    Arg::with_name("archive")
                        .index(1)
                        .value_name("archive")
                        .help("archive file to extract from")
                        .takes_value(true)
                        .required(true),
                )
                .arg(
                    Arg::with_name("output")
                        .value_name("dir")
                        .short("o")
                        .long("output")
                        .help("directory where to output extracted files")
                        .default_value("./")
                        .takes_value(true)
                )
                .arg(
                    Arg::with_name("filter")
                        .value_name("pattern")
                        .long("filter")
                        .short("f")
                        .help("if specified, will only extract files which match the specified glob-pattern")
                        .takes_value(true)
                )
        )
        .subcommand(
            SubCommand::with_name("view")
                .about("views a single file in an archive")
                .arg(
                    Arg::with_name("archive")
                        .index(1)
                        .value_name("archive")
                        .help("archive file to extract from")
                        .takes_value(true)
                        .required(true),
                )
                .arg(
                    Arg::with_name("file")
                        .index(2)
                        .value_name("filename")
                        .help("file inside the archive to view")
                        .takes_value(true)
                        .required(true)
                )
        )
        .get_matches_safe();

    let result = match matches {
        Err(error) => error.exit(),
        Ok(matches) => match matches.subcommand() {
            ("extract", Some(matches)) => command_extract(matches),
            ("view", Some(matches)) => command_view(matches),
            ("create", Some(matches)) => command_create(matches),
            (cmd, _) => {
                eprintln!("Unknown subcommand {} encountered", cmd);
                std::process::exit(1)
            }
        },
    };
}

fn command_extract(matches: &ArgMatches) -> Result<(), ()> {
    Ok(())
}

fn command_view(matches: &ArgMatches) -> Result<(), ()> {
    Ok(())
}

fn command_create(matches: &ArgMatches) -> Result<(), ()> {
    Ok(())
}
