pkgdatadir = join_paths(get_option('prefix'), get_option('datadir'), meson.project_name())

global_conf = configuration_data()
global_conf.set_quoted('APP_ID', application_id)
global_conf.set_quoted('PKGDATADIR', pkgdatadir)
global_conf.set_quoted('PROFILE', profile)
global_conf.set_quoted('VERSION', version + version_suffix)
global_conf.set_quoted('GETTEXT_PACKAGE', gettext_package)
global_conf.set_quoted('LOCALEDIR', localedir)
config_output_file = configure_file(
    input: 'config.rs.in',
    output: 'config.rs',
    configuration: global_conf
)

# Copy the config.rs output to the source directory.
run_command(
  'cp',
  join_paths(meson.build_root(), 'src', 'config.rs'),
  join_paths(meson.source_root(), 'src', 'config.rs'),
  check: true
)

sources = files(
  'backup/andotp.rs',
  'backup/bitwarden.rs',
  'backup/freeotp.rs',
  'backup/legacy.rs',
  'backup/mod.rs',
  'models/account_sorter.rs',
  'models/account.rs',
  'models/accounts.rs',
  'models/algorithm.rs',
  'models/database.rs',
  'models/favicon.rs',
  'models/i18n.rs',
  'models/keyring.rs',
  'models/mod.rs',
  'models/otp.rs',
  'models/otp_uri.rs',
  'models/provider_sorter.rs',
  'models/provider.rs',
  'models/providers.rs',
  'widgets/accounts/add.rs',
  'widgets/accounts/details.rs',
  'widgets/accounts/mod.rs',
  'widgets/accounts/qrcode_paintable.rs',
  'widgets/accounts/row.rs',
  'widgets/preferences/mod.rs',
  'widgets/preferences/password_page.rs',
  'widgets/preferences/window.rs',
  'widgets/providers/dialog.rs',
  'widgets/providers/image.rs',
  'widgets/providers/list.rs',
  'widgets/providers/mod.rs',
  'widgets/providers/page.rs',
  'widgets/providers/row.rs',
  'widgets/camera.rs',
  'widgets/error_revealer.rs',
	'widgets/mod.rs',
	'widgets/progress_icon.rs',
	'widgets/url_row.rs',
	'widgets/window.rs',
	'application.rs',
  'config.rs',
  'main.rs',
  'schema.rs',
  'window_state.rs'
)

cargo_script = find_program(join_paths(meson.source_root(), 'build-aux/cargo.sh'))
cargo_release = custom_target(
  'cargo-build',
  build_by_default: true,
  input: sources,
  output: meson.project_name(),
  console: true,
  install: true,
  install_dir: get_option('bindir'),
  command: [
    cargo_script,
    meson.build_root(),
    meson.source_root(),
    '@OUTPUT@',
    get_option('buildtype'),
    meson.project_name(),
  ]
)

cargo_target_dir = meson.build_root() / 'target'
cargo_home = cargo_target_dir / 'cargo-home'
test (
  'Cargo tests',
  cargo,
  args: ['test'],
  timeout: 300, # cargo might take a bit of time sometimes
  env: ['CARGO_TARGET_DIR=@0@'.format(cargo_target_dir), 'CARGO_HOME=@0@'.format(cargo_home)]
)

