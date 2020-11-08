ui_config = configuration_data()
ui_config.set('app-id', application_id)
ui_config.set('version', meson.project_version() + version_suffix)

ui_preconfigured_files = files(
  'resources/ui/about_dialog.ui.in',
  'resources/ui/providers_list.ui.in',
  'resources/ui/preferences.ui.in',
  'resources/ui/window.ui.in',
)
ui_dependencies = []
foreach ui_file: ui_preconfigured_files
  ui_dependencies += configure_file(
    input:  ui_file,
    output: '@BASENAME@',
    configuration: ui_config
  )
endforeach

resources = gnome.compile_resources(
  application_id,
  'resources.gresource.xml',
  gresource_bundle: true,
  dependencies: ui_dependencies
)


# GSchema
gschema_conf = configuration_data()
gschema_conf.set('app-id', application_id)
gschema_conf.set('gettext-package', gettext_package)
configure_file(
    input: 'com.belmoussaoui.Authenticator.gschema.xml.in',
    output: '@0@.gschema.xml'.format(application_id),
    configuration: gschema_conf,
    install: true,
    install_dir: datadir / 'glib-2.0' / 'schemas'
)

# Validata GSchema
if glib_compile_schemas.found()
  test(
    'validate-gschema', glib_compile_schemas,
    args: [
      '--strict', '--dry-run', meson.current_source_dir()
    ]
  )
endif


# GNOME Shell Search Provider Service
search_service_provider_conf = configuration_data()
search_service_provider_conf.set('app-id', application_id)
configure_file(
  configuration: search_service_provider_conf,
  input: 'com.belmoussaoui.Authenticator.SearchProvider.service.in',
  install_dir: get_option('datadir') / 'gnome-shell' / 'search-providers',
  output: '@0@.SearchProvider.service'.format(application_id)
)
# GNOME Shell Search Provider
search_provider_conf = configuration_data()
search_provider_conf.set('app-id', application_id)
search_provider_conf.set('profile', profile)
configure_file(
  configuration: search_provider_conf,
  input: 'com.belmoussaoui.Authenticator.SearchProvider.ini.in',
  install_dir: join_paths(get_option('datadir'), 'gnome-shell', 'search-providers'),
  output: '@0@.SearchProvider.ini'.format(application_id)
)

# FreeDesktop Desktop File
desktop_conf = configuration_data()
desktop_conf.set('icon', application_id)
desktop_file = i18n.merge_file(
  'desktop',
  input: configure_file(
    input: 'com.belmoussaoui.Authenticator.desktop.in.in',
    output: '@BASENAME@',
    configuration: desktop_conf
  ),
  output: '@0@.desktop'.format(application_id),
  po_dir: podir,
  type: 'desktop',
  install: true,
  install_dir: get_option('datadir') / 'applications'
)
# Validate Desktop File
desktop_file_validate = find_program('desktop-file-validate', required: false)
if desktop_file_validate.found()
  test (
    'Validate desktop file',
    desktop_file_validate,
    args: desktop_file.full_path()
  )
endif

# Freedesktop AppData File
appdata_conf = configuration_data()
appdata_conf.set('app-id', application_id)
appdata_conf.set('gettext-package', gettext_package)
appdata_file = i18n.merge_file(
  'appdata',
  input: configure_file(
    input: 'com.belmoussaoui.Authenticator.appdata.xml.in.in',
    output: '@BASENAME@',
    configuration: appdata_conf
  ),
  output: '@0@.appdata.xml'.format(application_id),
  po_dir: podir,
  install: true,
  install_dir: get_option('datadir') / 'metainfo'
)
# Validate AppData File
appstream_util = find_program('appstream-util', required: false)
if appstream_util.found()
  test (
    'Validate appdata file',
    appstream_util,
    args: ['validate', '--nonet', appdata_file.full_path()]
  )
endif

subdir('icons')
