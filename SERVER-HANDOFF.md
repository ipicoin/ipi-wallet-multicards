# IPI Card Vault — handoff dla serwera IPI Testnet

Stan handoffu: 2026-09-06. Dokument jest samowystarczalną instrukcją dla
operatora albo agenta działającego na serwerze noda. Celem jest zapisanie kodu
`ipi-card-vault` na `ipi-testnet-1`, zweryfikowanie go na łańcuchu i zwrócenie
`code_id` do konfiguracji IPI Wallet.

Ta procedura dotyczy wyłącznie Testnetu. Po migracji IPI do Mainnetu kontrakt
musi zostać ponownie zweryfikowany i zapisany na Mainnecie z serwera noda,
zgodnie z bramką opisaną w `README.md`. Nie wolno zastępować samych nazw sieci
w poniższych komendach ani używać testnetowego `code_id` na Mainnecie.

## Oczekiwany rezultat

Serwer ma zwrócić pięć wartości:

1. hash transakcji `store`;
2. `code_id`;
3. SHA-256 kodu odczytany z łańcucha;
4. hash i adres kontraktu utworzonego w smoke teście;
5. potwierdzenie, że zapytanie `config` zwraca `denom: aipi` oraz
   `member_count: 1`, a feegrant istnieje dla pierwszej karty.

## Co zostało przygotowane

Kontrakt realizuje jeden wspólny adres kontrolowany przez równoprawne karty
`1-of-N`:

- karty nie mają numerów, etykiet ani uprzywilejowanych ról;
- każda aktywna karta może przelewać środki, zapraszać kolejną kartę,
  anulować zaproszenie i usuwać inną kartę;
- zaproszenie nie zależy od dalszej obecności karty, która je utworzyła;
- ostatniej aktywnej karty nie można usunąć;
- zaproszona karta musi sama zaakceptować zaproszenie swoim kluczem;
- vault sponsoruje wywołania CosmWasm aktywnych kart przez feegrant;
- zaproszona karta otrzymuje ograniczony feegrant oraz `1 aipi`, aby nowy
  adres kontrolera mógł istnieć na łańcuchu;
- aplikacja tworzy instancje bez administratora, więc ich kodu nie można
  później podmienić kluczem zewnętrznym.

Najważniejsze pliki implementacji:

- `src/contract.rs` — członkostwo i operacje vaulta;
- `src/feegrant.rs` — granty ograniczone do
  `/cosmwasm.wasm.v1.MsgExecuteContract`;
- `src/msg.rs` — publiczny interfejs kontraktu;
- `artifacts/ipi_card_vault.wasm` — wersjonowany artefakt wdrożeniowy;
- `SERVER-HANDOFF.md` — niniejsza procedura operatorska.

Integracja Electron, UI, sieć i `IPI_CARD_VAULT_CODE_ID` należą do osobnego
repozytorium `ipi-wallet`. Przed pobraniem przez serwer oba zestawy zmian trzeba
zatwierdzić i wypchnąć do właściwych repozytoriów albo przesłać kontrolowaną
metodą. Nie wdrażać kontraktu z innego commita.

## Zweryfikowany artefakt

Lokalny artefakt:

```text
artifacts/ipi_card_vault.wasm
```

Parametry:

```text
rozmiar: 228947 bytes
SHA-256: fcb5a4653163ca4e3613390bb93c009da1fc49541db21bb11fdea5f64bfec454
optimizer: cosmwasm/optimizer:0.16.1
CosmWasm capability: cosmwasm_2_0
```

Katalog `artifacts/` i plik Wasm są celowo wersjonowane w tym repozytorium.
Serwer może pobrać plik razem z repozytorium albo odtworzyć go z dokładnie tego
samego commita. Nie wolno zatwierdzać uploadu, jeśli checksum się różni.

Odtworzenie artefaktu na serwerze, uruchomione z katalogu głównego repozytorium:

```sh
docker run --rm -v "$(pwd)":/code \
  --mount type=volume,source=ipi_card_vault_cache,target=/target \
  --mount type=volume,source=ipi_cosmwasm_registry_cache,target=/usr/local/cargo/registry \
  cosmwasm/optimizer:0.16.1

sha256sum artifacts/ipi_card_vault.wasm
```

Nie używać optimizera `0.17.x`, ponieważ emituje kontrakty wymagające
CosmWasm 3.x.

## Testy wykonane przed handoffem

Zakończone powodzeniem:

- `npm run check`;
- `npm test` — testy Vitest, KDF, Python i browser guard;
- `npm run build`;
- `cargo +1.85.0 test --locked` — 11 testów;
- release build dla `wasm32-unknown-unknown`;
- build przez `cosmwasm/optimizer:0.16.1`;
- `git diff --check`.

To nie zastępuje pierwszego smoke testu na IPI Testnet.

## Zasady bezpieczeństwa wdrożenia

- Użyć osobnego, zasilonego konta deployera. Nigdy nie używać klucza
  konsensusowego walidatora ani nie kopiować jego sekretu.
- Nie zapisywać mnemonic, hasła keyringu ani klucza prywatnego w tym pliku,
  historii powłoki, repozytorium lub logach CI.
- Najpierw sprawdzić synchronizację noda i uprawnienia `code_upload_access`.
- `instantiate_default_permission` albo per-code instantiate permission musi
  pozwalać kartom użytkowników tworzyć instancje. Wymagane jest `Everybody`.
- Nie ustawiać administratora podczas smoke testu ani podczas instancjowania z
  aplikacji.
- Nie zmieniać `IPI_CARD_VAULT_CODE_ID` przed pełną weryfikacją transakcji,
  checksumy i smoke testu.

## 1. Ustawienie parametrów na serwerze

Operator musi wpisać rzeczywistą nazwę binarki i istniejącego klucza. Poniższe
nazwy zmiennych są tylko dla tej sesji powłoki:

```sh
export IPI_NODE_BIN=/pełna/ścieżka/do/binarki-noda
export IPI_DEPLOYER_KEY=ipi-vault-deployer
export IPI_KEYRING_BACKEND=os
export IPI_CHAIN_ID=ipi-testnet-1
export IPI_NODE_RPC=tcp://127.0.0.1:26657
export IPI_GAS_PRICE=1500000000aipi
export IPI_ARTIFACT=/pełna/ścieżka/do/ipi_card_vault.wasm
export IPI_EXPECTED_SHA256=fcb5a4653163ca4e3613390bb93c009da1fc49541db21bb11fdea5f64bfec454
```

Nie zakładać, że binarka nazywa się `ipid` lub `wasmd`. Odczytać `ExecStart`
rzeczywistej usługi noda albo użyć binarki wskazanej przez operatora.

## 2. Preflight tylko do odczytu

```sh
"$IPI_NODE_BIN" version --long
"$IPI_NODE_BIN" status --node "$IPI_NODE_RPC"
"$IPI_NODE_BIN" query wasm params \
  --node "$IPI_NODE_RPC" \
  --output json

export IPI_DEPLOYER_ADDRESS="$(
  "$IPI_NODE_BIN" keys show "$IPI_DEPLOYER_KEY" \
    --keyring-backend "$IPI_KEYRING_BACKEND" \
    --address
)"

"$IPI_NODE_BIN" query bank balances "$IPI_DEPLOYER_ADDRESS" \
  --node "$IPI_NODE_RPC" \
  --output json

sha256sum "$IPI_ARTIFACT"
```

Przerwać, jeżeli:

- node raportuje inną sieć niż `ipi-testnet-1` albo nadal się synchronizuje;
- konto deployera nie ma wystarczającego salda `aipi`;
- deployer nie ma prawa wykonać `MsgStoreCode`;
- domyślne uprawnienie instancjowania nie jest `Everybody` i binarka nie
  umożliwia jawnego ustawienia tej wartości dla przechowywanego kodu;
- lokalny SHA-256 nie jest równy oczekiwanej wartości.

Przed transakcją należy wykonać:

```sh
"$IPI_NODE_BIN" tx wasm store --help
```

Jeżeli help udostępnia flagę ustawiającą instantiate permission na `Everybody`,
należy jej użyć. Nazwa flagi zależy od dokładnej wersji binarki; nie należy jej
zgadywać.

## 3. Zapisanie kodu na IPI Testnet

Najpierw uruchomić symulację, jeśli dana binarka ją obsługuje. Następnie:

```sh
"$IPI_NODE_BIN" tx wasm store "$IPI_ARTIFACT" \
  --from "$IPI_DEPLOYER_KEY" \
  --keyring-backend "$IPI_KEYRING_BACKEND" \
  --chain-id "$IPI_CHAIN_ID" \
  --node "$IPI_NODE_RPC" \
  --gas auto \
  --gas-adjustment 1.4 \
  --gas-prices "$IPI_GAS_PRICE" \
  --broadcast-mode sync \
  --output json \
  --yes
```

Jeżeli krok 2 wykazał potrzebę jawnej flagi instantiate permission, dodać do
tej komendy flagę potwierdzoną przez `tx wasm store --help`.

Zapisać `txhash` z odpowiedzi. Nie uznawać odpowiedzi trybu `sync` za
potwierdzenie wykonania. Po wejściu transakcji do bloku:

```sh
export IPI_STORE_TXHASH=WKLEJ_HASH_TRANSAKCJI

"$IPI_NODE_BIN" query tx "$IPI_STORE_TXHASH" \
  --node "$IPI_NODE_RPC" \
  --output json
```

Wynik musi mieć `code: 0`. Z eventu `store_code` odczytać atrybut `code_id`:

```sh
export IPI_CODE_ID=WKLEJ_CODE_ID_Z_EVENTU

"$IPI_NODE_BIN" query wasm code-info "$IPI_CODE_ID" \
  --node "$IPI_NODE_RPC" \
  --output json
```

Pole `data_hash` z `code-info`, zapisane małymi literami, musi być równe
`IPI_EXPECTED_SHA256`. Należy również potwierdzić, że instantiate permission
dla kodu jest `Everybody`.

## 4. Obowiązkowy smoke test na łańcuchu

Testowa instancja ma powstać bez administratora i bez środków. Flaga w typowej
binarce wasmd nazywa się `--no-admin`; przed użyciem potwierdzić ją przez
`tx wasm instantiate --help`.

```sh
"$IPI_NODE_BIN" tx wasm instantiate "$IPI_CODE_ID" '{}' \
  --label "ipi-card-vault-smoke-$IPI_CODE_ID" \
  --no-admin \
  --from "$IPI_DEPLOYER_KEY" \
  --keyring-backend "$IPI_KEYRING_BACKEND" \
  --chain-id "$IPI_CHAIN_ID" \
  --node "$IPI_NODE_RPC" \
  --gas auto \
  --gas-adjustment 1.4 \
  --gas-prices "$IPI_GAS_PRICE" \
  --broadcast-mode sync \
  --output json \
  --yes
```

Zapisać hash, poczekać na blok i sprawdzić `query tx`. Wynik musi mieć
`code: 0`. Z eventu `instantiate` odczytać `_contract_address`, a potem:

```sh
export IPI_SMOKE_CONTRACT=WKLEJ_ADRES_Z_EVENTU

"$IPI_NODE_BIN" query wasm contract "$IPI_SMOKE_CONTRACT" \
  --node "$IPI_NODE_RPC" \
  --output json

"$IPI_NODE_BIN" query wasm contract-state smart "$IPI_SMOKE_CONTRACT" \
  '{"config":{}}' \
  --node "$IPI_NODE_RPC" \
  --output json

"$IPI_NODE_BIN" query wasm contract-state smart "$IPI_SMOKE_CONTRACT" \
  "{\"member\":{\"address\":\"$IPI_DEPLOYER_ADDRESS\"}}" \
  --node "$IPI_NODE_RPC" \
  --output json

"$IPI_NODE_BIN" query feegrant grant \
  "$IPI_SMOKE_CONTRACT" \
  "$IPI_DEPLOYER_ADDRESS" \
  --node "$IPI_NODE_RPC" \
  --output json
```

Oczekiwane wyniki:

- informacje o kontrakcie wskazują dokładnie wdrożone `code_id`;
- instancja nie ma administratora;
- `config` zwraca `aipi` i `member_count: 1`;
- zapytanie `member` zwraca adres deployera;
- feegrant ma grantera równego adresowi testowego vaulta, grantee równego
  adresowi deployera i dopuszcza wyłącznie
  `/cosmwasm.wasm.v1.MsgExecuteContract`.

Jeżeli lokalna binarka nie posiada komendy `query feegrant grant`, użyć
odpowiadającego jej endpointu REST Cosmos SDK. Brak feegrantu oznacza
nieudany smoke test nawet wtedy, gdy instancja powstała.

Nie usuwać testowej instancji: kontraktów CosmWasm nie da się usunąć, a pusty
vault bez środków nie wymaga dalszej obsługi.

## 5. Przekazanie wyniku do IPI Wallet

Po powodzeniu wszystkich kontroli serwer przekazuje operatorowi portfela:

```text
network=ipi-testnet-1
store_txhash=...
code_id=...
on_chain_sha256=fcb5a4653163ca4e3613390bb93c009da1fc49541db21bb11fdea5f64bfec454
smoke_txhash=...
smoke_contract=...
smoke_config_ok=true
smoke_feegrant_ok=true
```

Portfel należy uruchamiać z:

```sh
export IPI_CARD_VAULT_CODE_ID=CODE_ID_ZWERYFIKOWANY_POWYŻEJ
npm start
```

Jeżeli aplikacja jest uruchamiana jako usługa, ustawić tę samą zmienną w
kontrolowanym środowisku usługi i zrestartować wyłącznie aplikację portfela.
Nie zmieniać konfiguracji noda.

## 6. Test użytkowy dwóch kart

Po ustawieniu `IPI_CARD_VAULT_CODE_ID`:

1. Kartą A utworzyć nowy wspólny portfel i zasilić adres vaulta.
2. Odczytać adres kontrolera karty B.
3. Kartą A wybrać dodanie karty i podać adres kontrolera B.
4. Kartą B otworzyć adres wspólnego portfela i zaakceptować zaproszenie.
5. Potwierdzić, że obie karty widzą ten sam adres i to samo saldo.
6. Wykonać mały przelew kartą B i potwierdzić, że opłatę sponsoruje vault.
7. Kartą B zaprosić kartę C albo usunąć kartę A, aby potwierdzić pełną
   symetrię uprawnień. Nie wykonywać tego kroku z realnymi środkami.

Kryterium akceptacji: każda aktywna karta zachowuje się identycznie; żadna nie
jest kartą główną, zapasową ani kartą o określonym numerze.

## Warunki przerwania i raport błędu

Przerwać dalszą konfigurację portfela i przekazać pełny, zanonimizowany wynik,
jeżeli wystąpi którykolwiek z przypadków:

- checksum lokalny i on-chain są różne;
- `store`, `instantiate` albo podzapytanie zwraca niezerowy kod;
- sieć odrzuca capability `cosmwasm_2_0`;
- `MsgGrantAllowance` lub `MsgRevokeAllowance` jest odrzucany;
- instancja ma administratora;
- permission nie pozwala dowolnej karcie utworzyć instancji;
- feegrant zawiera szerszy typ wiadomości niż `MsgExecuteContract`;
- aplikacja rozpoznaje inny code ID niż wdrożony.

Raport nie może zawierać mnemonic, haseł, prywatnych kluczy ani zawartości
keyringu. Wystarczą: wersja binarki, wysokość bloku, txhash, code ID, adres
kontraktu, nieudana komenda i publiczny błąd łańcucha.
