FROM alpine:latest

RUN apk add --no-cache grub-bios xorriso

CMD ["grub-mkrescue", "-d", "/usr/lib/grub/i386-pc", "-o", "/build/osmium.iso", "/build/isoroot"]
