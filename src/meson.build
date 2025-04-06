pkgdatadir = get_option('prefix') / get_option('datadir') / meson.project_name()
moduledir = pkgdatadir / 'noty'
gnome = import('gnome')

blueprints = custom_target(
  'blueprints',
  input: files(
    'gtk/help-overlay.blp',
    'ui/note_list_item.blp',
    'ui/window.blp',
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
conf.set('VERSION', meson.project_version())
conf.set('localedir', get_option('prefix') / get_option('localedir'))
conf.set('pkgdatadir', pkgdatadir)

configure_file(
  input: 'noty.in',
  output: 'noty',
  configuration: conf,
  install: true,
  install_dir: get_option('bindir'),
  install_mode: 'r-xr-xr-x',
)

noty_sources_main = [
  '__init__.py',
  'main.py',
  'window.py',
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

install_data(noty_sources_main, install_dir: moduledir)

install_data(noty_sources_services, install_dir: moduledir / 'services')

install_data(noty_sources_models, install_dir: moduledir / 'models')

install_data(noty_sources_widgets, install_dir: moduledir / 'widgets')