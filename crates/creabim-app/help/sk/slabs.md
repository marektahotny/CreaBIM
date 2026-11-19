# Dosky

**Doska** je podlaha, strop, terasa alebo balkón: plochý prvok na podlaží nakreslený podľa svojho obrysu.

## Kreslenie dosky {#drawing-a-slab}

Zvoľte nástroj **Doska**, potom režim vo voľbách nástroja (alebo jeho číselný kláves):

- **Mnohouholník** (`1`): kliknite na každý roh, potom znova na prvý roh alebo `Enter`. `A` kreslí oblúk (kliknite na jeho koniec, potom na bod, cez ktorý prechádza) a `L` vráti rovné čiary.
- **Obdĺžnik** (`2`): kliknite na dva protiľahlé rohy.
- **Vyplniť miestnosť** (`3`): kliknite dovnútra miestnosti; doska ju vyplní po steny.
- **Z náčrtu** (`4`): kliknite na uzavretý [náčrt](sketches).

`Esc` alebo pravé tlačidlo vráti posledný bod a ďalšie opustí nástroj. Rohy sa zarovnávajú s predchádzajúcimi: čiarkované čiary ukazujú, keď je ukazovateľ vodorovne alebo zvisle s rohom.

### Uzavretie obrysu {#closing-the-outline}

Každý nástroj obrysu (dosky, strechy, podhľady, šachty, oblasti, orezové obrysy a obrysy na ploche) sa uzatvára rovnako:

- priblížte ukazovateľ k prvému rohu: jeho značka narastie do krúžku **Uzavrieť** a tvar sa vyplní; kliknite naň;
- stlačte `C` alebo `Enter`, dvakrát kliknite, alebo kliknite na **Uzavrieť** vedľa *Čiara* a *Oblúk* vo voľbách nástroja.

`Backspace` vráti posledný bod a stavový riadok body počíta. Bod pustený na predchádzajúci sa nikdy nepridá dvakrát.

V 3D pohľade možno dosky, strechy a šachty kresliť aj na podlaží alebo na ploche, ako mnohouholník alebo **Obdĺžnik**.

![Doska s oblúkom](img/slab-arc.jpg)

V reze kliknite na dva konce hrany dosky: jej vrch sa prichytí k najbližšiemu podlažiu.

## Zmena dosky {#changing-a-slab}

- Ťahajte jej úchyty rohov a hrán. Rohy pridáte či odstránite v [Režime úprav](edit-mode) (`Enter`): vyberte roh a stlačte `Delete`, potiahnite ◇ a pridáte roh, pravým tlačidlom na hranu ju ohnete do oblúka.
- *Vlastnosti*: hrúbka, materiál alebo skladba, *Odsadenie vrchu* a *Ponechať: Vrch / Spodok* (ktorá plocha zostane pri zmene hrúbky na mieste).
- *Prekresliť obrys* a kliknutím zadajte nový obrys.
- V reze použite [Natiahnuť](align#stretch-to-it) alebo [Posun plochy](move#move-a-face-push-and-pull) na jej vrchu alebo spodku.

## Otvory {#holes}

*Kresliť otvor* vo *Vlastnostiach* v nej vyreže otvor; [šachta](holes#shafts) reže každú dosku, ktorou prechádza. Pozri [Otvory](holes).
