# rsplit

rsplit BIN/IVF/WEBM/YUV/PSNR/...

Usage: 

rsplit bin input.bin output_prefix frame_num h264|h265

rsplit ivf input.ivf output_prefix frame_num vp8|vp9

rsplit webm input.webm output.ivf frame_num vp8|vp9

rsplit yuv input.yuv output_prefix frame_num frame_size1 [...|frame_size2 ...]

rsplit psnr input1.yuv input2.yuv frame_num frame_size1 [...|frame_size2 ...]

===

how to build nestegg static library:

1). g++ -fPIC -c src/nestegg/*.cpp

2). ar rvs libnestegg.a halloc.o nestegg.o vpx.o

2.1) nm -g libnestegg.a    (just to make sure that no name-mangling)

3). export LIBRARY_PATH="./"

4). cargo build (or cargo run)