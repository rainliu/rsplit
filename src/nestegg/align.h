#ifndef _LIBP_ALIGN_H_
#define _LIBP_ALIGN_H_

/*
 *	a type with the most strict alignment requirements
 */
typedef union libp_align
{
	char   c;
	short  s;
	long   l;
	int    i;
	float  f;
	double d;
	void * v;
	void (*q)(void);
}libp_align_t;

#endif

