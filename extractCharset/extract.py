from PIL import Image
from bitarray import bitarray

file = Image.open("charset.png").convert("RGB")
w, h = file.size
image = file.load()
with open("charset.bin", "wb") as out:
    bitarray("""
        00000000
        00000000
        00000000
        00000000
        00000000
        00000000
        00000000
        00000000
        00000000

        01111111
        01111111
        01111111
        01111111
        01111111
        01111111
        01111111
        01111111
        01111111

        01111000
        01111000
        01111000
        01111000
        01111000
        01111000
        01111000
        01111000
        01111000

        00001111
        00001111
        00001111
        00001111
        00001111
        00001111
        00001111
        00001111
        00001111

        01111111
        01111111
        01111111
        01111111
        00000000
        00000000
        00000000
        00000000
        00000000

        00000000
        00000000
        00000000
        00000000
        01111111
        01111111
        01111111
        01111111
        01111111

        01111111
        01111111
        01111111
        01111111
        01111111
        00000000
        00000000
        00000000
        00000000

        00000000
        00000000
        00000000
        00000000
        00000000
        01111111
        01111111
        01111111
        01111111

        01111000
        01111000
        01111000
        01111000
        00000000
        00000000
        00000000
        00000000
        00000000

        00001111
        00001111
        00001111
        00001111
        00000000
        00000000
        00000000
        00000000
        00000000
        
        00000000
        00000000
        00000000
        00000000
        00000000
        01111000
        01111000
        01111000
        01111000

        00000000
        00000000
        00000000
        00000000
        00000000
        00001111
        00001111
        00001111
        00001111
    """ + "0" * 20 * 9 * 8).tofile(out)
    y = 0
    ycount = 1
    while y < h and ycount < 7:
        x = 0
        xcount = 1
        while x < w and xcount < 19:
            for cy in range(9):
                byte = 0
                for cx in range(7):
                    if image[x + cx, y + cy] != (0, 0, 0):
                        byte |= 1 << cx
            
                out.write(byte.to_bytes(1, "little"))
            x += 7
            xcount += 1

            if ycount == 6 and xcount == 6: break
        y += 9
        ycount += 1