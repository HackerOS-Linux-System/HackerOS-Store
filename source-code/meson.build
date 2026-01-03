project('HackerOS-Store', 'vala', 'c',
  version: '0.5',
  default_options: ['warning_level=3'])

gnome = import('gnome')

gtk4_dep = dependency('gtk4')
glib_dep = dependency('glib-2.0')
gobject_dep = dependency('gobject-2.0')

dependencies = [
  gtk4_dep,
  glib_dep,
  gobject_dep,
]

gresource_xml = configure_file(
  output: 'hackeros.gresource.xml',
  configuration: configuration_data(),
  input: files('hackeros.gresource.xml.in')
)

resource_data = files('images/Hacker-Unpack.png')

resources = gnome.compile_resources(
  'hackeros_resources',
  gresource_xml,
  dependencies: resource_data,
  source_dir: 'images',
  c_name: 'hackeros'
)

sources = files('main.vala') + resources

executable('hackeros-store', sources,
  dependencies: dependencies,
  install: true)
