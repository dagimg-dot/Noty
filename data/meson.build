desktop_file = i18n.merge_file(
        input: 'com.dagimg.noty.desktop.in',
       output: 'com.dagimg.noty.desktop',
         type: 'desktop',
       po_dir: '../po',
      install: true,
  install_dir: get_option('datadir') / 'applications'
)

desktop_utils = find_program('desktop-file-validate', required: false)
if desktop_utils.found()
  test('Validate desktop file', desktop_utils, args: [desktop_file])
endif

appstream_file = i18n.merge_file(
        input: 'com.dagimg.noty.metainfo.xml.in',
       output: 'com.dagimg.noty.metainfo.xml',
       po_dir: '../po',
      install: true,
  install_dir: get_option('datadir') / 'metainfo'
)

appstreamcli = find_program('appstreamcli', required: false, disabler: true)
test('Validate appstream file', appstreamcli,
     args: ['validate', '--no-net', '--explain', appstream_file])

install_data('com.dagimg.noty.gschema.xml',
  install_dir: get_option('datadir') / 'glib-2.0' / 'schemas'
)

compile_schemas = find_program('glib-compile-schemas', required: false, disabler: true)
test('Validate schema file',
     compile_schemas,
     args: ['--strict', '--dry-run', meson.current_source_dir()])


service_conf = configuration_data()
service_conf.set('bindir', get_option('prefix') / get_option('bindir'))

# Get application_id from parent scope
application_id = get_option('application_id')

# Use the appropriate service file based on application_id
if application_id == 'com.dagimg.dev.noty'
  service_input = 'com.dagimg.dev.noty.service.in'
  service_output = 'com.dagimg.dev.noty.service'
else
  service_input = 'com.dagimg.noty.service.in'
  service_output = 'com.dagimg.noty.service'
endif

configure_file(
  input: service_input,
  output: service_output,
  configuration: service_conf,
  install_dir: get_option('datadir') / 'dbus-1' / 'services'
)

subdir('icons')
