#include <assert.h>
#include <string.h>
#include <stdlib.h>
#include <stdio.h>
#include "macros.h"
#include "nestegg.h"
#include "vpx.h"

typedef struct {
  unsigned char 	signature[4]; //='DKIF'; 
  unsigned short	version;      //= 0;
  unsigned short	headerlength; //= 32; 
  unsigned int    FourCC; 
  unsigned short	width; 
  unsigned short	height; 
  unsigned int    framerate; 
  unsigned int    timescale; 
  unsigned int    framenum; 
  unsigned char 	unused[4]; 
}IVF_SEQUENCE_HEADER; 

#pragma pack(4)
typedef struct {
  unsigned int      frameSize; 
  UInt64            timeStamp; 	
}IVF_FRAME_HEADER; 
#pragma pack()

#define VP8_FOURCC (0x00385056)
#define VP9_FOURCC (0x00395056)

enum file_kind {
  RAW_FILE,
  IVF_FILE,
  WEBM_FILE
};

struct input_ctx {
  FILE           *infile;
  
  nestegg        *nestegg_ctx;
  nestegg_packet *pkt;
  unsigned int    chunk;
  unsigned int    chunks;
  unsigned int    video_track;

  bool                 m_is_ivf;
  bool                 m_is_webm;
  const unsigned char *m_superframes_data_start[8];
  unsigned int         m_superframes_data_sz[8];
  int                  m_superframes_data_count;
  int                  m_superframes_data_idx;
};

static unsigned int mem_get_le16(const void *vmem) 
{
  unsigned int  val;
  const unsigned char *mem = (const unsigned char *)vmem;

  val  = mem[1] << 8;
  val |= mem[0];
  return val;
}

static unsigned int mem_get_le32(const void *vmem) 
{
  unsigned int  val;
  const unsigned char *mem = (const unsigned char *)vmem;

  val  = mem[3] << 24;
  val |= mem[2] << 16;
  val |= mem[1] << 8;
  val |= mem[0];
  return val;
}

unsigned int file_is_ivf(FILE *infile,
                         unsigned int *fourcc,
                         unsigned int *width,
                         unsigned int *height,
                         unsigned int *fps_den,
                         unsigned int *fps_num) 
{
  char raw_hdr[32];
  int is_ivf = 0;

  if (fread(raw_hdr, 1, 32, infile) == 32) {
    if (raw_hdr[0] == 'D' && raw_hdr[1] == 'K'
        && raw_hdr[2] == 'I' && raw_hdr[3] == 'F') {
      is_ivf = 1;

      if (mem_get_le16(raw_hdr + 4) != 0)
        fprintf(stderr, "Error: Unrecognized IVF version! This file may not decode properly.");

      *fourcc = mem_get_le32(raw_hdr + 8);
      *width = mem_get_le16(raw_hdr + 12);
      *height = mem_get_le16(raw_hdr + 14);
      *fps_num = mem_get_le32(raw_hdr + 16);
      *fps_den = mem_get_le32(raw_hdr + 20);

      /* Some versions of vpxenc used 1/(2*fps) for the timebase, so
       * we can guess the framerate using only the timebase in this
       * case. Other files would require reading ahead to guess the
       * timebase, like we do for webm.
       */
      if (*fps_num < 1000) {
        /* Correct for the factor of 2 applied to the timebase in the
         * encoder.
         */
        if (*fps_num & 1)*fps_den <<= 1;
        else *fps_num >>= 1;
      } else {
        /* Don't know FPS for sure, and don't have readahead code
         * (yet?), so just default to 30fps.
         */
        *fps_num = 30;
        *fps_den = 1;
      }
    }
  }

  if (!is_ivf)
    rewind(infile);

  return is_ivf;
}


int file_is_webm(struct input_ctx *input,
                 unsigned int     *fourcc,
                 unsigned int     *width,
                 unsigned int     *height,
                 unsigned int     *fps_den,
                 unsigned int     *fps_num) 
{
  unsigned int i, n;
  int          track_type = -1;
  int          codec_id;

  nestegg_io io = {nestegg_read_cb, nestegg_seek_cb, nestegg_tell_cb, 0};
  nestegg_video_params params;

  io.userdata = input->infile;
  if (nestegg_init(&input->nestegg_ctx, io, NULL))
    goto fail;

  if (nestegg_track_count(input->nestegg_ctx, &n))
    goto fail;

  for (i = 0; i < n; i++) {
    track_type = nestegg_track_type(input->nestegg_ctx, i);

    if (track_type == NESTEGG_TRACK_VIDEO)
      break;
    else if (track_type < 0)
      goto fail;
  }

  codec_id = nestegg_track_codec_id(input->nestegg_ctx, i);
  if (codec_id == NESTEGG_CODEC_VP8) {
    *fourcc = VP8_FOURCC;
  } else if (codec_id == NESTEGG_CODEC_VP9) {
    *fourcc = VP9_FOURCC;
  } else {
    printf("Not VPx video, quitting.\n");
    return 0;
  }

  input->video_track = i;

  if (nestegg_track_video_params(input->nestegg_ctx, i, &params))
    goto fail;

  *fps_den = 0;
  *fps_num = 0;
  *width = params.width;
  *height = params.height;  
  return 1;
fail:
  input->nestegg_ctx = NULL;
  rewind(input->infile);
  return 0;
}

unsigned int get_one_frame_from_ivf(unsigned char **buf, struct input_ctx *input)
{
  unsigned int len = 0;
  IVF_FRAME_HEADER  sIvfFrmHdr;

  // Dec IVF header
  fread(&sIvfFrmHdr, sizeof(IVF_FRAME_HEADER), 1, input->infile);

  if(input->pkt!=NULL){
    nestegg_free_packet(input->pkt);
  }
  input->pkt = (nestegg_packet *)malloc(sizeof(nestegg_packet));
  input->pkt->frame = (struct frame * )malloc(sizeof(struct frame));
  input->pkt->frame->next=NULL;
  input->pkt->frame->length = sIvfFrmHdr.frameSize;
  input->pkt->frame->data = (unsigned char * )malloc(sIvfFrmHdr.frameSize);
  *buf = input->pkt->frame->data;
  len = (unsigned int)(fread(input->pkt->frame->data, sizeof(unsigned char), sIvfFrmHdr.frameSize, input->infile));
  if (len != sIvfFrmHdr.frameSize)
  {
    if(!feof(input->infile)){
      printf("Read Buffer Failed!\n");
    }
    return 0;
  }

  return len;
}

unsigned int get_one_frame_from_webm(unsigned char **buf, struct input_ctx *input)
{
  size_t buf_sz;
  FILE *infile = input->infile;

  if (input->chunk >= input->chunks) {
    unsigned int track;

    do {
      /* End of this packet, get another. */
      if (input->pkt){
        nestegg_free_packet(input->pkt);
        input->pkt = NULL;
      }

      if (nestegg_read_packet(input->nestegg_ctx, &input->pkt) <= 0
        || nestegg_packet_track(input->pkt, &track)){   
        if(!feof(input->infile)){
          printf("Read Buffer Failed!"); 
        }
        return 0;
      }

    } while (track != input->video_track);

    if (nestegg_packet_count(input->pkt, &input->chunks)){
      if(!feof(input->infile)){
        printf("Read Buffer Failed!");
      }
      return 0;
    }
    input->chunk = 0;
  }

  if (nestegg_packet_data(input->pkt, input->chunk, buf, &buf_sz)){
    if(!feof(input->infile)){
      printf("Read Buffer Failed!");
    }
    return 0;
  }

  input->chunk++;

  return (unsigned int)buf_sz;
}

void parse_superframe_index(const unsigned char *data,
                            size_t               data_sz,
                            unsigned int         sizes[8],
                            int                 *count) 
{
  unsigned char marker;

  assert(data_sz);
  marker = data[data_sz - 1];
  *count = 0;

  if ((marker & 0xe0) == 0xc0) {
    const unsigned int frames = (marker & 0x7) + 1;
    const unsigned int mag = ((marker >> 3) & 0x3) + 1;
    const size_t index_sz = 2 + mag * frames;

    if (data_sz >= index_sz && data[data_sz - index_sz] == marker) {
      // found a valid superframe index
      unsigned int i, j;
      const unsigned char *x = data + data_sz - index_sz + 1;

      for (i = 0; i < frames; i++) {
        unsigned int this_sz = 0;

        for (j = 0; j < mag; j++)
          this_sz |= (*x++) << (j * 8);
        sizes[i] = this_sz;
      }

      *count = frames;
    }
  }
}

int decode_superframes(const unsigned char *data,
                       unsigned int         data_sz,
                       const unsigned char *superframes_data_start[8],
                       unsigned int         superframes_data_sz[8]) 
{
  const unsigned char *data_start = data;
  const unsigned char *data_end = data + data_sz;  
  unsigned int sizes[8];
  int frames_this_pts = 0, frame_count = 0;

  parse_superframe_index(data, data_sz, sizes, &frames_this_pts);

  do {
    // Skip over the superframe index, if present
    if (data_sz && (*data_start & 0xe0) == 0xc0) {
        const unsigned char marker = *data_start;
        const unsigned int frames = (marker & 0x7) + 1;
        const unsigned int mag = ((marker >> 3) & 0x3) + 1;
        const unsigned int index_sz = 2 + mag * frames;

        if (data_sz >= index_sz && data_start[index_sz - 1] == marker) {
            data_start += index_sz;
            data_sz -= index_sz;
            if (data_start < data_end)
                continue;
            else
                break;
        }
    }

    // Use the correct size for this frame, if an index is present.
    if (frames_this_pts) {
      unsigned int this_sz = sizes[frame_count];

      if (data_sz < this_sz) {
        printf("Invalid frame size in index");
        return 0;//exit(-1);
      }

      data_sz = this_sz;
    }     
    superframes_data_start[frame_count] = data_start;
    superframes_data_sz   [frame_count] = data_sz;
    frame_count++;   

    /* Account for suboptimal termination by the encoder. */
    //while (data_start < data_end && *data_start == 0)
    //  data_start++;
    data_start += data_sz;
    data_sz = (unsigned int)(data_end - data_start);
  } while (data_start < data_end);  

  return frame_count;
}

void* vpx_init(const char* filename)
{
    FILE *fp = fopen(filename, "rb");
    if(fp==NULL){
        return NULL;
    }
  
    struct input_ctx *input = (struct input_ctx *)malloc(sizeof(struct input_ctx));
    memset((void *)input, 0, sizeof(struct input_ctx));

    unsigned int fourcc;
    unsigned int width;
    unsigned int height;
    unsigned int fps_den;
    unsigned int fps_num;   

    input->infile = fp;
    if (file_is_ivf(input->infile, &fourcc, &width, &height, &fps_den, &fps_num))
        input->m_is_ivf = 1;
    else if (file_is_webm(input, &fourcc, &width, &height, &fps_den, &fps_num))
        input->m_is_webm = 1;

    return (void *)input;
}

const unsigned char* vpx_read(void *userdata, unsigned int *nBufSize)
{
  struct input_ctx *input = (struct input_ctx *)userdata;
  *nBufSize = 0;

  if (input->m_is_ivf || input->m_is_webm){    
    if (input->m_superframes_data_idx == input->m_superframes_data_count){
      unsigned int len = 0;
      unsigned char *buf = NULL;
      if (input->m_is_ivf) { 
          len = get_one_frame_from_ivf(&buf, input); 
      }
      else if (input->m_is_webm) {
          len = get_one_frame_from_webm(&buf, input);
      }

      if (len == 0) return NULL;
      input->m_superframes_data_count = decode_superframes(buf, len, input->m_superframes_data_start, input->m_superframes_data_sz);
      input->m_superframes_data_idx = 0;
    }
    //assert(input->m_superframes_data_sz[input->m_superframes_data_idx] <= nBufSize);
    //memcpy(pBuf, input->m_superframes_data_start[input->m_superframes_data_idx], input->m_superframes_data_sz[input->m_superframes_data_idx]);
    const unsigned char *pBuf = input->m_superframes_data_start[input->m_superframes_data_idx];
    *nBufSize  = input->m_superframes_data_sz[input->m_superframes_data_idx];
    input->m_superframes_data_idx++;
    return pBuf;
  }
  else{
    printf("Unrecognized input file type.\n");
    return NULL;
  }
}

void vpx_destroy(void *userdata)
{
  if(userdata!=NULL){
    struct input_ctx *input = (struct input_ctx *)userdata;
    if (input->pkt){
        nestegg_free_packet(input->pkt);
        input->pkt = NULL;
    }
    if (input->nestegg_ctx){
        nestegg_destroy(input->nestegg_ctx);
        input->nestegg_ctx = NULL;
    }
    free(input);
  }
}