pkgdatadir = get_option('prefix') / get_option('datadir') / meson.project_name()
moduledir = pkgdatadir / 'noty'
gnome = import('gnome')

# Get application ID from options
application_id = get_option('application_id')
app_version = meson.project_version()

blueprints = custom_target(
  'blueprints',
  input: files(
    'gtk/help-overlay.blp',
    'ui/note_list_item.blp',
    'ui/window.blp',
    'ui/preferences.blp',
  ),
  output: '.',
  command: [
    find_program('blueprint-compiler'),
    'batch-compile',
    '@OUTPUT@',
    '@CURRENT_SOURCE_DIR@',
    '@INPUT@',
  ],
)

gnome.compile_resources(
  'noty',
  'noty.gresource.xml',
  gresource_bundle: true,
  install: true,
  install_dir: pkgdatadir,
  dependencies: blueprints,
)

python = import('python')

conf = configuration_data()
conf.set('PYTHON', python.find_installation('python3').full_path())
conf.set('VERSION', app_version)
conf.set('localedir', get_option('prefix') / get_option('localedir'))
conf.set('pkgdatadir', pkgdatadir)
conf.set('application_id', application_id)

configure_file(
  input: 'noty.in',
  output: 'noty',
  configuration: conf,
  install: true,
  install_dir: get_option('bindir'),
  install_mode: 'r-xr-xr-x',
)

configure_file(
  input: '__init__.py',
  output: '__init__.py',
  configuration: conf,
  install: true,
  install_dir: moduledir,
)

configure_file(
  input: 'main.py',
  output: 'main.py',
  configuration: conf,
  install: true,
  install_dir: moduledir,
)

noty_sources_main = [
  'windows/window.py',
  'windows/preferences.py',
]

noty_sources_services = [
  'services/conf_manager.py',
  'services/file_manager.py',
]

noty_sources_models = [
  'models/note.py',
]

noty_sources_widgets = [
  'widgets/note_list_item.py',
]

noty_sources_utils = [
  'utils/constants.py',
]

install_data(noty_sources_main, install_dir: moduledir / 'windows')

install_data(noty_sources_services, install_dir: moduledir / 'services')

install_data(noty_sources_models, install_dir: moduledir / 'models')

install_data(noty_sources_widgets, install_dir: moduledir / 'widgets')

install_data(noty_sources_utils, install_dir: moduledir / 'utils')