project('HackerOS-Store', 'vala', 'c',
  version: '0.1',
  default_options: ['warning_level=3'])

gtk4_dep = dependency('gtk4')
glib_dep = dependency('glib-2.0')
gobject_dep = dependency('gobject-2.0')

dependencies = [
  gtk4_dep,
  glib_dep,
  gobject_dep,
]

sources = files('hackeros_store.vala')

executable('hackeros-store', sources,
  dependencies: dependencies,
  install: true)
