<p align="center">
  <a href="#ipi-card-vault--wspólny-portfel-wielu-kart"><img src="docs/assets/multicards-mark.svg" width="208" alt="Kilka fizycznych kart IPI połączonych z jednym skarbcem"></a>
  &nbsp;&nbsp;&nbsp;&nbsp;
  <a href="#ipi-payment-relay--pięcioetapowe-przekazanie-płatności"><img src="docs/assets/privacy-relay-mark.svg" width="208" alt="Zanonimizowana postać otoczona pięcioma przekaźnikami płatności"></a>
</p>

<h1 align="center">IPI Wallet Multicards</h1>

<p align="center">
  <strong>Wiele kart. Jeden skarbiec. Pięć przekaźników.</strong><br>
  Gotowe kontrakty CosmWasm dla współdzielonego portfela IPI Wallet.
</p>

---

`ipi-wallet-multicards` jest repozytorium kontraktów CosmWasm używanych przez
IPI Wallet do obsługi jednego portfela przez wiele fizycznych kart oraz do
przekazywania płatności przez warstwę pięciu przekaźników.

Repozytorium zawiera kod źródłowy obu kontraktów, zablokowane wersje zależności,
testy, dane wdrożeniowe oraz gotowe, zoptymalizowane artefakty Wasm z sumami
kontrolnymi. Nie zawiera aplikacji desktopowej, oprogramowania kart ani procesu
wydawania kart. Nie jest też usługą serwerową: operator zapisuje artefakt w
wybranym łańcuchu, a kontrakt działa później bezpośrednio w tym łańcuchu.
Aplikacja `ipi-wallet` korzysta ze zweryfikowanego, osobnego dla każdej sieci
`code_id`.

> **Status bezpieczeństwa:** to nieaudytowane oprogramowanie laboratoryjne.
> Nie powinno chronić środków o istotnej wartości.

## Co znajduje się w repozytorium

| Element | Plik | Rozmiar | SHA-256 |
| --- | --- | ---: | --- |
| IPI Card Vault | `contracts/multicarding/ipi_card_vault.wasm` | 228947 B | `fcb5a4653163ca4e3613390bb93c009da1fc49541db21bb11fdea5f64bfec454` |
| IPI Payment Relay | `contracts/anonymization/ipi_payment_relay.wasm` | 236954 B | `3ce3c4124544daf100f483110be4c5c9093d4f035fe14259bd2e3ceaef761553` |

Najważniejsze katalogi:

```text
card-vault/
├── Cargo.toml
└── src/                     # źródła IPI Card Vault
payment-relay/
├── Cargo.toml
├── src/                     # źródła IPI Payment Relay
└── tests/                   # testy integracyjne pełnej trasy
Cargo.toml                   # wspólny workspace obu kontraktów
Cargo.lock                   # wspólna blokada zależności
contracts/
├── multicarding/
│   ├── ipi_card_vault.wasm
│   └── checksums.txt
└── anonymization/
    ├── ipi_payment_relay.wasm
    └── checksums.txt
deployments/                 # zapisane i szablonowe dane wdrożeń
```

Integralność plików należy sprawdzić przed każdym wdrożeniem:

```sh
(cd contracts/multicarding && sha256sum --check checksums.txt)
(cd contracts/anonymization && sha256sum --check checksums.txt)
```

Nie wolno wdrażać artefaktu, jeżeli jego suma jest inna. Artefakty zbudowano
dla CosmWasm 2.x. Sieć docelowa musi obsługiwać `cosmwasm_2_0`, natywny nominał
`aipi`, wiadomości protobuf `Any` oraz moduł `feegrant`.

## Budowanie i testy

Projekt korzysta z Rust `1.85.0`, wskazanego w `rust-toolchain.toml`. Oba
kontrakty są równorzędnymi pakietami jednego workspace i można je sprawdzić
niezależnie:

```sh
cargo test --locked -p ipi-card-vault
cargo test --locked -p ipi-payment-relay
cargo clippy --locked -p ipi-card-vault --all-targets -- -D warnings
cargo clippy --locked -p ipi-payment-relay --all-targets -- -D warnings
```

CI dodatkowo buduje oba cele `wasm32-unknown-unknown`, odtwarza zoptymalizowane
artefakty przy użyciu `cosmwasm/optimizer:0.16.1` i porównuje ich bajty z
plikami w `contracts/`.

## IPI Card Vault — wspólny portfel wielu kart

Card Vault jest kontraktem przechowującym środki i jednocześnie wspólnym
adresem odbiorczym portfela. Każda dołączona karta zachowuje własny klucz
prywatny i własny adres kontrolera. Kontrakt nie przechowuje kluczy kart.

Model uprawnień to pełne **1-z-N**, a nie multisig progowy:

- karta tworząca skarbiec staje się pierwszym członkiem, ale nie otrzymuje
  roli właściciela ani innych szczególnych uprawnień;
- każda aktywna karta może samodzielnie przesłać środki ze skarbca, zaprosić
  kolejną kartę, anulować dowolne oczekujące zaproszenie i usunąć dowolną
  aktywną kartę, również samą siebie;
- zaproszona karta musi zaakceptować zaproszenie transakcją podpisaną własnym
  kluczem;
- nie można usunąć ostatniej aktywnej karty;
- zaproszenie należy do całego skarbca, a nie tylko do karty, która je
  utworzyła.

Każda aktywna karta otrzymuje finansowany przez skarbiec feegrant ograniczony
do wiadomości `/cosmwasm.wasm.v1.MsgExecuteContract`. Zaproszony, wcześniej
nieużywany adres otrzymuje `1 aipi` i ograniczony feegrant potrzebny do
zaakceptowania zaproszenia. Anulowanie zaproszenia lub usunięcie karty odwołuje
odpowiadający mu feegrant.

Kontrakt przyjmuje przy instancjonowaniu wyłącznie `aipi`; przesłane wtedy
środki stają się saldem skarbca. Wywołania wykonawcze nie przyjmują dołączonych
środków. Wartość przelewu musi być dodatnia i jest zawsze wyrażona w `aipi`.

### Instancjonowanie

Kod powinien mieć uprawnienie instancjonowania `Everybody`, a każdą instancję
należy tworzyć bez administratora. Komunikat instancjonowania jest pusty:

```json
{}
```

Nadawca transakcji zostaje pierwszą aktywną kartą. Brak administratora jest
celowy: uniemożliwia późniejszą podmianę kodu instancji za pomocą migracji.

### Komunikaty wykonawcze

```json
{"invite_card":{"address":"ipi1..."}}
{"accept_invitation":{}}
{"cancel_invitation":{"address":"ipi1..."}}
{"remove_card":{"address":"ipi1..."}}
{"transfer":{"recipient":"ipi1...","amount":"123"}}
```

`amount` jest liczbą całkowitą przekazaną jako tekst. Operacje zaproszenia,
anulowania, usuwania i przelewu podlegają opisanej wyżej kontroli członkostwa.

### Zapytania

```json
{"config":{}}
{"member":{"address":"ipi1..."}}
{"members":{"start_after":null,"limit":30}}
{"invitation":{"address":"ipi1..."}}
{"invitations":{"start_after":null,"limit":30}}
```

`config` zwraca nominał i liczbę aktywnych kart. Zapytania pojedyncze zwracają
członka lub zaproszenie, jeśli istnieje. Listy są stronicowane po adresie;
domyślny limit wynosi 30, a maksymalny 100.

## IPI Payment Relay — pięcioetapowe przekazanie płatności

Payment Relay oddziela adres Card Vault od adresu widocznego jako bezpośredni
nadawca końcowego przelewu bankowego. Dla każdego skarbca portfel tworzy pięć
osobnych, niezmiennych instancji tego samego kodu, o unikalnych slotach `0..4`.
Każda instancja jest powiązana z jednym Card Vault, nie przechowuje klucza i po
prawidłowej płatności nie zatrzymuje salda.

Typowa płatność przygotowana przez `ipi-wallet` jest jedną atomową transakcją:

1. Card Vault przekazuje dokładną kwotę do aktywnego kontrolera karty.
2. Kontroler przekazuje ją do pierwszego z pięciu przekaźników.
3. Wszystkie pięć przekaźników przekazuje całość w losowej kolejności.
4. Ostatni przekaźnik wysyła całą kwotę do odbiorcy.

Pierwszy przekaźnik wymaga dokładnie czterech dalszych, unikalnych instancji.
Każda z nich musi mieć ten sam `code_id`, nie mieć administratora, należeć do
tego samego skarbca i reprezentować inny slot. Jeżeli którykolwiek etap się
nie powiedzie, cała transakcja zostaje wycofana.

### Ograniczenie prywatności

Payment Relay **nie zapewnia kryptograficznej anonimowości** i nie jest
mikserem. Łańcuch publicznie ujawnia skarbiec, kontroler, kwotę, odbiorcę,
identyfikator płatności oraz całą trasę. Mechanizm ogranicza jedynie proste,
bezpośrednie powiązanie adresu skarbca z nadawcą widocznym w podstawowym
interfejsie odbiorcy. Każdy analizujący dane łańcucha może odtworzyć trasę.

### Instancjonowanie

Artefakt zapisuje się w łańcuchu tylko raz, z uprawnieniem `Everybody`. Dla
każdego skarbca aktywny kontroler tworzy pięć instancji bez administratora:

```json
{"vault":"ipi1...","slot":0}
```

Operację należy powtórzyć dla slotów `1`, `2`, `3` i `4`. Kontrakt odrzuca
środki podczas instancjonowania, slot spoza zakresu oraz próbę wykonaną przez
adres, który nie jest aktywnym członkiem wskazanego Card Vault.

Zapytanie konfiguracyjne:

```json
{"config":{}}
```

Powinno zwrócić właściwy `vault`, unikalny `slot`, `denom: "aipi"` oraz
`relay_count: 5`.

### Przekazanie płatności

```json
{
  "forward": {
    "controller": "ipi1...",
    "route": ["ipi1relay...", "ipi1relay...", "ipi1relay...", "ipi1relay..."],
    "recipient": "ipi1...",
    "payment_id": "0000000000000000000000000000000000000000000000000000000000000000"
  }
}
```

Pierwsze wywołanie musi pochodzić od aktywnego kontrolera, zawierać dokładnie
cztery pozostałe przekaźniki i przenosić dokładnie jedną dodatnią monetę
`aipi`. `payment_id` musi być 64-znakową, małymi literami zapisaną wartością
szesnastkową. Kolejne wywołania mogą pochodzić wyłącznie od zgodnych instancji
przekaźnika. Saldo każdej instancji po prawidłowej płatności powinno wynosić
zero.

## Zarejestrowany stan wdrożeń

IPI Card Vault został zweryfikowany na `ipi-testnet-1`:

- `code_id`: `2`;
- transakcja zapisania kodu:
  `3116E3BAC8EA64DCDCBE7FAB78B42E75C2DEED63F07A8BD2E71893174832438F`;
- kontrakt testowy:
  `ipi1nc5tatafv6eyq7llkr2gv50ff9e22mnf70qgjlv737ktmt4eswrq5df8fy`;
- administrator instancji: brak;
- SHA-256 kodu w łańcuchu:
  `fcb5a4653163ca4e3613390bb93c009da1fc49541db21bb11fdea5f64bfec454`.

Payment Relay nie ma w tym repozytorium potwierdzonego wdrożenia na
`ipi-testnet-1`. Funkcja powinna pozostać wyłączona w aplikacji, dopóki nie
zostaną zarejestrowane i niezależnie sprawdzone co najmniej: transakcja store,
on-chain SHA-256, `code_id`, uprawnienie `Everybody`, brak administratora oraz
pełny test pięciu instancji.

Powyższe identyfikatory dotyczą wyłącznie `ipi-testnet-1`. `code_id`, hash
transakcji ani adres kontraktu nie są przenośne między Testnetem a Mainnetem.
Wdrożenie Mainnet wymaga ponownego zapisania i sprawdzenia obu artefaktów oraz
skonfigurowania portfela identyfikatorami uzyskanymi z Mainnetu.

## Minimalna kontrola wdrożenia

Przed udostępnieniem kontraktu aplikacji operator powinien:

1. użyć zsynchronizowanego węzła oraz osobnego, zasilonego konta operacyjnego;
2. sprawdzić chain ID, nominał `aipi`, możliwości runtime i lokalne SHA-256;
3. zapisać dokładnie śledzony w Git artefakt Wasm z uprawnieniem `Everybody`;
4. poczekać na zatwierdzenie transakcji i potwierdzić kod wykonania `0`;
5. pobrać informację o kodzie i porównać on-chain SHA-256 z `checksums.txt`;
6. utworzyć testową instancję bez administratora i sprawdzić jej konfigurację;
7. dla Card Vault sprawdzić pierwszego członka i ograniczony feegrant;
8. dla Payment Relay sprawdzić pięć slotów, pełną trasę, zmianę salda odbiorcy
   o dokładną kwotę i zerowe saldo wszystkich przekaźników;
9. przekazać aplikacji wyłącznie `code_id` zweryfikowane dla właściwej sieci.

Nie należy używać klucza konsensusu walidatora ani wdrażać lokalnie
przebudowanego, nieśledzonego artefaktu.

## Najważniejsze konsekwencje bezpieczeństwa

- Utrata jednej aktywnej karty wraz z jej poświadczeniem może oznaczać utratę
  całego wspólnego salda. Dodanie karty tworzy kolejną pełną ścieżkę autoryzacji.
- Instancje bez administratora nie mogą zostać zaktualizowane migracją. Nowa
  wersja wymaga nowego kodu i nowych instancji.
- Nominał `aipi` jest zapisany w kontraktach na stałe. Zmiana nominału sieci
  wymaga nowego artefaktu.
- Dane Payment Relay są jawne i umożliwiają pełne prześledzenie płatności.
- Przekaźnik nie ma funkcji odzyskania środków. `aipi` wysłane zwykłym przelewem
  na jego adres zamiast przez `forward` mogą pozostać na nim na stałe.

## Licencja

Kod i artefakty są własnością IPI. Szczegółowe warunki znajdują się w pliku
[`LICENSE`](LICENSE).
