#ifndef _LIBP_VPX_H_
#define _LIBP_VPX_H_

#ifdef __cplusplus
extern "C" {
#endif

void* vpx_init(const char* filename);

const unsigned char* vpx_read(void *input, unsigned int *length);

void vpx_destroy(void *input);

#ifdef __cplusplus
}
#endif

#endif