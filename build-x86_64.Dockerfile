FROM alpine:latest

RUN apk add --no-cache grub-bios grub-efi mtools xorriso

CMD ["grub-mkrescue", "-o", "/build/osmium.iso", "/build/isoroot"]
