resources = gnome.compile_resources(
  meson.project_name(),
  'resources.gresource.xml',
  gresource_bundle: true,
  install: true,
  install_dir: pkgdatadir,
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

# FreeDesktop Desktop File
desktop_conf = configuration_data()
desktop_conf.set('icon', application_id)
desktop_file = i18n.merge_file(
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
metainfo_conf = configuration_data()
metainfo_conf.set('app-id', application_id)
metainfo_conf.set('gettext-package', gettext_package)
metainfo_file = i18n.merge_file(
  input: configure_file(
    input: 'com.belmoussaoui.Authenticator.metainfo.xml.in.in',
    output: '@BASENAME@',
    configuration: metainfo_conf
  ),
  output: '@0@.metainfo.xml'.format(application_id),
  po_dir: podir,
  install: true,
  install_dir: get_option('datadir') / 'metainfo'
)
# Validate metainfo File
appstream_util = find_program('appstream-util', required: false)
if appstream_util.found()
  test (
    'Validate metainfo file',
    appstream_util,
    args: ['validate-relax', '--nonet', metainfo_file.full_path()]
  )
endif


# Search Provider
service_conf = configuration_data()
service_conf.set('appid', application_id)
service_conf.set('name', meson.project_name())
service_conf.set('bindir', bindir)
configure_file(
  input: 'com.belmoussaoui.Authenticator.SearchProvider.service.in',
  output: '@0@.SearchProvider.service'.format(application_id),
  configuration: service_conf,
  install_dir: datadir / 'dbus-1' / 'services'
)
search_conf = configuration_data()
search_conf.set('appid', application_id)
search_conf.set('object_path', object_path)
configure_file(
  input: 'com.belmoussaoui.Authenticator.search-provider.ini',
  output: '@0@.search-provider.ini'.format(application_id),
  configuration: search_conf,
  install_dir: datadir / 'gnome-shell' / 'search-providers',
)

subdir('icons')
