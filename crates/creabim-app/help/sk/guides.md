# Pomocné čiary

## Dočasné pomocné čiary {#temporary-guide-lines}

Keď je aktívny kresliaci nástroj, z ktoréhokoľvek bodu alebo hrany môžu vychádzať čiarkované pomocné čiary na zarovnanie, ako sledovanie v iných CAD programoch.

1. Podržte ukazovateľ asi pol sekundy na bode alebo hrane, alebo nad ním stlačte `Q`. Malá modrá značka ukazuje, že je **zachytený**.
2. Vytvorí pomocné čiary:
   - bod: vodorovnú a zvislú čiaru a čiary v každom kroku uhla (predvolene 45°);
   - hrana: jej predĺženie a kolmice na jej koncoch;
   - po kliknutí na prvý bod: rovnobežky a kolmice cez neho a uhly od neho;
   - oblúk alebo kružnica: jej kružnica, polomery a dotyčnice.
3. Ukazovateľ sa prichytáva k pomocným čiaram a najmä k miestam, kde sa dve pretínajú. Popis ich pomenuje, napríklad *Predĺženie · Kolmica*.
4. Opätovným podržaním na tej istej položke ju zrušíte.

![Pomocné čiary zo zachyteného rohu](img/guides.jpg)

- `Esc` bez rozpracovaných kliknutí vymaže všetky pomocné čiary.
- `K` ponechá rovnú pomocnú čiaru pod ukazovateľom ako skutočnú pomocnú čiaru.
- Ponuka **Pomocné čiary** vo voľbách nástroja ich zapína a vypína, vyberá krok uhla (len vodorovne a zvislo, 15°, 30° alebo 45°) a vymaže ich.

Fungujú v pôdorysoch, rezoch a pohľadoch.

## Nástroj Pomocná čiara {#the-guide-tool}

**Pomocné čiary** sú konštrukčné čiary uložené v modeli: čiarkované, prichytávajú sa k nim, nikdy sa netlačia ani neexportujú.

- Zvoľte nástroj **Pomocná čiara** a kliknite na dva body, cez ktoré prechádza.
- Alebo kliknite na hranu a potom tam, kam má ísť rovnobežná pomocná čiara.

V 3D sa pomocné čiary kreslia na [pracovnú rovinu](sketches#the-workplane); v reze na rovinu rezu.

*Pohľad › Pomocné čiary* (`Shift+G`) skryje všetky pomocné čiary vo všetkých pohľadoch bez ich odstránenia.
