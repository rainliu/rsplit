#ifndef _LIBP_MACROS_H_
#define _LIBP_MACROS_H_

#include <stddef.h>  /* offsetof */

/*
 	restore pointer to the structure by a pointer to its field
 */
#define structof(p,t,f) ((t*)(- (ptrdiff_t) offsetof(t,f) + (char*)(p)))

/*
 *	redefine for the target compiler
 */
#ifdef _WIN32
#define static_inline static __inline
#else
#define static_inline static __inline__
#endif

#ifdef _WIN32
typedef __int64                     Int64;
typedef unsigned __int64            UInt64;
#else
typedef long long                   Int64;
typedef unsigned long long          UInt64;
#endif

#endif

