include $(top_srcdir)/config/Rules.am

lib_LTLIBRARIES = libuutil.la

USER_C = \
	uu_alloc.c \
	uu_avl.c \
	uu_dprintf.c \
	uu_ident.c \
	uu_list.c \
	uu_misc.c \
	uu_open.c \
	uu_pname.c \
	uu_string.c

libuutil_la_SOURCES = $(USER_C)

libuutil_la_LIBADD = \
	$(top_builddir)/lib/libavl/libavl.la \
	$(top_builddir)/lib/libspl/libspl.la

if BUILD_FREEBSD
libuutil_la_LDFLAGS = -pthread -version-info 3:0:0
else
libuutil_la_LDFLAGS = -pthread -version-info 1:1:0
endif
