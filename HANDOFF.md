# HANDOFF — wdrożenie kontraktów IPI Wallet

To repozytorium dystrybucyjne zawiera wyłącznie gotowe artefakty CosmWasm dla
portfela IPI oraz informacje potrzebne do ich bezpiecznego wdrożenia. Nie
zawiera kodu źródłowego ani lokalnych wyników kompilacji.

## Artefakty

| Funkcja | Plik | Rozmiar | SHA-256 |
| --- | --- | ---: | --- |
| Multicarding (Card Vault) | `contracts/multicarding/ipi_card_vault.wasm` | 228947 B | `fcb5a4653163ca4e3613390bb93c009da1fc49541db21bb11fdea5f64bfec454` |
| Anonimizacja (Payment Relay) | `contracts/anonymization/ipi_payment_relay.wasm` | 236954 B | `3ce3c4124544daf100f483110be4c5c9093d4f035fe14259bd2e3ceaef761553` |

Przed każdym wdrożeniem sprawdź bajty przeznaczone do wysłania:

```sh
(cd contracts/multicarding && sha256sum --check checksums.txt)
(cd contracts/anonymization && sha256sum --check checksums.txt)
```

Nie wdrażaj pliku, jeśli suma jest inna. Artefakty zbudowano dla CosmWasm 2.x;
łańcuch musi obsługiwać `cosmwasm_2_0`, wiadomości protobuf `Any`, feegrant oraz
natywny nominał `aipi`.

## 1. Multicarding — IPI Card Vault

Card Vault jest wspólnym adresem odbiorczym kontrolowanym przez aktywne karty.
Każda karta zachowuje własny klucz i ma równorzędne uprawnienia 1-z-N: może
przesyłać środki, zapraszać i usuwać karty oraz anulować zaproszenia. Ostatniej
aktywnej karty nie można usunąć. Kontrakt finansuje ograniczony feegrant dla
zaproszenia i feegrant aktywnej karty ograniczony do wywołań CosmWasm.

Artefakt należy zapisać w łańcuchu z uprawnieniem instancjonowania `Everybody`.
Każdą instancję twórz bez administratora (`--no-admin`). Komunikat
instancjonowania to:

```json
{}
```

Nadawca zostaje pierwszą aktywną kartą. Podczas instancjonowania można dołączyć
wyłącznie `aipi`; przekazane środki stają się saldem skarbca.

Interfejs wykonawczy:

```json
{"invite_card":{"address":"ipi1..."}}
{"accept_invitation":{}}
{"cancel_invitation":{"address":"ipi1..."}}
{"remove_card":{"address":"ipi1..."}}
{"transfer":{"recipient":"ipi1...","amount":"123"}}
```

Dostępne zapytania: `config`, `member`, `members`, `invitation` i `invitations`.

Zweryfikowane wdrożenie na `ipi-testnet-1`:

- code ID: `2`;
- suma Wasm: `fcb5a4653163ca4e3613390bb93c009da1fc49541db21bb11fdea5f64bfec454`;
- transakcja store: `3116E3BAC8EA64DCDCBE7FAB78B42E75C2DEED63F07A8BD2E71893174832438F`;
- kontrakt smoke-test: `ipi1nc5tatafv6eyq7llkr2gv50ff9e22mnf70qgjlv737ktmt4eswrq5df8fy`;
- administrator instancji: brak.

Code ID jest właściwy wyłącznie dla tego Testnetu. Na Mainnecie trzeba wykonać
nowy store i przekazać portfelowi nowy, zweryfikowany code ID.

## 2. Anonimizacja — IPI Payment Relay

Payment Relay odsuwa adres skarbca od odbiorcy o pięć przekaźników. Dla każdego
skarbca portfel tworzy pięć osobnych, niezmiennych instancji o slotach `0..4`,
wszystkie z tego samego code ID i bez administratora. Instancję może utworzyć
tylko aktywny kontroler wskazanego Card Vault.

Najpierw zapisz Wasm raz w łańcuchu z uprawnieniem `Everybody`, a następnie dla
każdego slotu wywołaj bez administratora:

```json
{"vault":"ipi1...","slot":0}
```

Powtórz dla slotów `1`, `2`, `3` i `4`. Zapytanie `config` każdej instancji musi
zwrócić właściwy `vault`, unikalny `slot`, `denom: "aipi"` i `relay_count: 5`.
Saldo każdego przekaźnika po płatności powinno wynosić zero.

Wywołanie płatności ma postać:

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

Pierwszy przekaźnik otrzymuje dokładnie cztery pozostałe, unikalne adresy oraz
jedną dodatnią monetę `aipi`. Cała trasa działa atomowo.

Ważne: ten mechanizm nie zapewnia kryptograficznej anonimowości. Trasa, kwota,
kontroler, odbiorca i identyfikator płatności są publiczne w zdarzeniach
łańcucha. Mechanizm ogranicza jedynie bezpośrednie powiązanie adresu skarbca z
nadawcą widocznym w prostym interfejsie odbiorcy.

Payment Relay nie ma jeszcze potwierdzonego wdrożenia na `ipi-testnet-1`.
Po store należy zapisać i niezależnie sprawdzić hash transakcji, code ID,
on-chain SHA-256, uprawnienie `Everybody` oraz wynik testu pięciu instancji.

## Kontrola wdrożenia

Dla obu kontraktów operator powinien:

1. użyć zsynchronizowanego węzła i osobnego, zasilonego konta wdrożeniowego;
2. sprawdzić chain ID, nominał, saldo, wersję CosmWasm i lokalne SHA-256;
3. wysłać dokładnie śledzony plik Wasm;
4. poczekać na zatwierdzenie transakcji i potwierdzić `code: 0`;
5. porównać on-chain hash kodu z wartością z tabeli;
6. potwierdzić uprawnienie instancjonowania `Everybody`;
7. utworzyć instancję testową bez administratora i sprawdzić jej `config`;
8. przekazać aplikacji portfela code ID z właściwego łańcucha.

Nie używaj klucza konsensusu walidatora i nie kopiuj code ID między Testnetem a
Mainnetem. To nieaudytowane oprogramowanie laboratoryjne; nie powinno chronić
środków o istotnej wartości.

## Licencja

Kod i artefakty są własnością IPI. Warunki znajdują się w `LICENSE`.
