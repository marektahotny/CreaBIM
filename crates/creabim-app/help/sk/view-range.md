# Rozsah pohľadu

**Rozsah pohľadu** pôdorysu určuje, ktoré podlažia kreslí. Nastavíte ho v časti *Rozsah pohľadu* vo *Vlastnostiach* pôdorysu.

- *Jeho podlažie*: len prvky vlastného podlažia pôdorysu, prerezané v jeho *Výške rezu*. Takto sa kreslili pôdorysy pred zavedením rozsahov pohľadu.
- *Podlažia v rozsahu*: prvky všetkých podlaží medzi *Spodkom* a *Vrchom*.
- *Zhora*: pôdorys strechy alebo situácia, pohľad kolmo nadol bez rezu.

## Podlažia v rozsahu {#levels-in-range}

- **Spodok**: podlažie a odsadenie. Podlažia odtiaľto až po rovinu rezu sa kreslia ako vlastné pôdorysy: prerezané tam, kde nimi rovina rezu prechádza, v pohľade tam, kde ležia pod ňou. Úroveň terénu 0,45 m pod podlahou s ohradovými múrmi a vrátnicou sa v pôdoryse zobrazí s prerezanými stenami.
- **Vrch**: podlažia začínajúce nad ním sa vynechajú. Predvolene *Nasledujúce podlažie*.
- **Hĺbka pohľadu**: podlažia medzi ňou a spodkom sa kreslia svetlejšie, štýlom *za*.
- **Nad rezom čiarkovane**: to, čo leží nad rezom až po vrch, sa kreslí čiarkovane: okraje strechy, horná doska, hlava komína.

Na vlastnom podlaží pôdorysu sa časť ležiaca celá nad rezom, napríklad krycia doska nízkeho múrika, kreslí vždy čiarkovane ako nad rezom.

Každá výška je podlažie a odsadenie od neho, takže sa posúva spolu s podlažím.

Hĺbka pohľadu nemôže zobraziť to, čo pohľad skrýva: ak pôdorys skrýva základy, pod hĺbkou pohľadu sa to uvedie s tlačidlom **Zobraziť základy**.

## Rozsahy pôdorysov v rezoch {#plan-ranges-in-sections}

V hlavičke rezu alebo pohľadu zapnite **Rozsahy pôdorysov**: rez každého pôdorysu sa zobrazí plnou čiarou a jeho hĺbka pohľadu čiarkovanou, v páse širokom ako výkres. Sú nenápadné, kým na jeden neukážete. Potiahnutím úchopu na konci pásu zmeníte výšku rezu alebo hĺbku pohľadu, kliknutím na čiaru rezu zadáte jej výšku.

## Orezový kváder rezu {#section-box}

V časti *Rozsah pohľadu › Orezový kváder rezu* oreže pôdorys 3D pohľad tak, ako ho vidí: **Pod rezom** ponechá to, čo je pod rezom pôdorysu, **Nad (podhľady)** ponechá to, čo je nad ním, na pohľad nahor na podhľady. *Orezový kváder rezu › V 3D* pri reze oreže 3D pohľad od jeho čiary po jeho hĺbku. Každý z nich je [orezový kváder](crop-boxes) 3D pohľadu, aktualizovaný, keď o to znova požiadate.

## Zhora {#from-above}

- Pôdorys sa pozerá nadol od *Vrchu* (predvolene nad všetkým) po *Spodok* (predvolene pod všetkým).
- Nič nie je prerezané. Každý povrch zakrýva to, čo je pod ním: vyššie strechy zakrývajú nižšie, strechy zakrývajú steny a miestnosti pod nimi.
- Strechy zobrazujú šípky sklonu. Osi, značky rezov, kóty a poznámky sa zobrazujú ako v každom pôdoryse.
- Jeho šablóna môže pridať tiene od slnka a slabnutie so vzdialenosťou, pozri [Grafické štýly](graphic-styles#depth-cues).

S balíkom *Jemné čiary* dostanú pôdorysy zhora šablónu *Pôdorys strechy*.
