#ifndef _LIBP_HALLOC_H_
#define _LIBP_HALLOC_H_

#include <stddef.h>  /* size_t */

/*
 *	Core API
 */
void * halloc (void * block, size_t len);
void   hattach(void * block, void * parent);

/*
 *	standard malloc/free api
 */
void * h_malloc (size_t len);
void * h_calloc (size_t n, size_t len);
void * h_realloc(void * p, size_t len);
void   h_free   (void * p);
char * h_strdup (const char * str);

/*
 *	the underlying allocator
 */
typedef void * (* realloc_t)(void * ptr, size_t len);

extern realloc_t halloc_allocator;

#endif

