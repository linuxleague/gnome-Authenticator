project('authenticator', 'rust',
	version: '4.3.1',
  meson_version: '>= 0.59',
  license: 'GPL-3.0-or-later'
)

dependency('glib-2.0', version: '>= 2.56')
dependency('gio-2.0', version: '>= 2.56')
dependency('gdk-pixbuf-2.0')
dependency('gtk4', version: '>= 4.10')
dependency('libadwaita-1', version: '>=1.4')
dependency('zbar', version: '>= 0.20')
dependency('gstreamer-1.0', version: '>= 1.18')
dependency('gstreamer-base-1.0', version: '>= 1.18')
dependency('gstreamer-plugins-base-1.0', version: '>= 1.18')
dependency('gstreamer-plugins-bad-1.0', version: '>= 1.18')

cargo = find_program('cargo', required: true)
glib_compile_schemas = find_program('glib-compile-schemas', required: true)
glib_compile_resources = find_program('glib-compile-resources', required: true)

version = meson.project_version()

prefix = get_option('prefix')
bindir = prefix / get_option('bindir')
localedir = prefix / get_option('localedir')

datadir = prefix / get_option('datadir')
pkgdatadir = datadir / meson.project_name()
iconsdir = datadir / 'icons'
podir = meson.project_source_root () / 'po'
gettext_package = meson.project_name()

if get_option('profile') == 'development'
  application_id = 'com.belmoussaoui.Authenticator.Devel'
  profile = 'Devel'
  vcs_tag = run_command('git', 'rev-parse', '--short', 'HEAD').stdout().strip()
  if vcs_tag == ''
    version_suffix = '-devel'
  else
    version_suffix = '-@0@'.format (vcs_tag)
  endif
  object_path = '/com/belmoussaoui/Authenticator/Devel/SearchProvider'
else
  profile = ''
  version_suffix = ''
  application_id = 'com.belmoussaoui.Authenticator'
  object_path = '/com/belmoussaoui/Authenticator/SearchProvider'

endif

i18n = import('i18n')
gnome = import('gnome')

subdir('po')
subdir('data')
subdir('src')

meson.add_dist_script(
  'build-aux/dist-vendor.sh',
  meson.project_build_root() / 'meson-dist' / meson.project_name() + '-' + version,
  meson.project_source_root()
)

if get_option('profile') == 'development'
    # Setup pre-commit hook for ensuring coding style is always consistent
    message('Setting up git pre-commit hook..')
    run_command('cp', '-f', 'hooks/pre-commit.hook', '.git/hooks/pre-commit')
endif

gnome.post_install(
  glib_compile_schemas: true,
  gtk_update_icon_cache: true,
  update_desktop_database: true
)
