# Fiscalité des crypto-actifs (particuliers)

Voir `data/plus-values-mobilieres-crypto.json` → `crypto_actifs`.

## Régime des particuliers (occasionnel)

Le régime des particuliers s'applique aux cessions **occasionnelles** d'actifs numériques. Si l'activité est habituelle/professionnelle, requalification en **BIC** (cotisations sociales TNS, régime plus lourd).

**Indices d'activité habituelle** :
- Volume de transactions élevé
- Fréquence quasi quotidienne
- Usage d'outils professionnels (bots, API, arbitrage automatisé)
- Revenus crypto principaux du foyer

## Fait générateur

| Opération | Imposable ? |
|-----------|-------------|
| Achat crypto contre € / USD | Non |
| Cession crypto contre € / USD | **Oui** |
| Paiement en crypto (biens/services) | **Oui** (cession déguisée) |
| Échange crypto-to-crypto (BTC → ETH) | **Non** (sursis, art. 150 VH bis) |
| Staking / mining / airdrop | Selon contexte — souvent BNC ou BIC, pas PV mobilière |

**Règle du sursis crypto-to-crypto** : les échanges entre crypto-actifs ne déclenchent pas l'imposition. Seul le passage en monnaie fiat (ou en biens/services) est taxable.

## Méthode PAMC (Prix d'Acquisition Moyen Pondéré en Continu)

**Formule officielle** :

```
plus_value_cession = prix_cession − (valeur_totale_portefeuille × montant_cession / valeur_portefeuille_avant_cession)
```

**Conséquences pratiques** :
- Chaque cession puise dans le portefeuille global (pas en FIFO, pas en LIFO)
- Nécessite de tracer l'historique complet depuis le premier achat
- Si prix d'achat non documentés → risque de requalification en cession au prix 0 (PV max)

**Outils recommandés** : Koinly, CoinTracking, Waltio, Cryptio. À vérifier que le logiciel applique bien la PAMC française.

## Taux d'imposition

### Régime par défaut : PFU 30%

- 12,8% IR + 17,2% PS
- Application sur la plus-value nette annuelle (après compensation des moins-values de l'année)

### Option barème (depuis revenus 2023)

Possible depuis la LFI 2022 (applicable revenus 2023). **Avantageuse si TMI ≤ 11%.**

**Rappel** : l'option barème est **globale** — s'applique à tous les revenus du capital de l'année (y compris dividendes, intérêts, PV mobilières). Arbitrage à faire au niveau global.

## Exonération du petit portefeuille

**Seuil annuel : 305 €** de cessions cumulées.

- Cessions ≤ 305 € par an → **exonération totale**
- Cessions > 305 € par an → **imposition intégrale** (pas seulement la fraction au-delà)

**Piège** : le seuil s'applique sur le **montant brut des cessions** de l'année, pas sur la plus-value. Vendre 500 € de crypto avec une PV de 10 € déclenche l'imposition sur les 10 € de PV.

## Compensation des moins-values

Les moins-values de l'année sont **compensables** avec les plus-values de l'année (crypto uniquement, pas compensables avec PV mobilières classiques).

**Pas de report** des moins-values crypto sur les années suivantes (règle spécifique).

## Formulaire 2086

Déclaration obligatoire détaillant **chaque cession** :
- Date de la cession
- Valeur du portefeuille avant cession
- Prix total d'acquisition du portefeuille
- Prix de la cession
- Plus-value ou moins-value calculée

**Report sur 2042 C** :
- Case 3AN : plus-value nette annuelle (gain)
- Case 3BN : moins-value nette annuelle (perte)

## Staking, mining, airdrops

**Régime distinct des PV** — imposition selon la nature :

| Activité | Régime probable |
|----------|----------------|
| Staking occasionnel | BNC non professionnel ou PV mobilière selon cas |
| Mining | BIC |
| Staking/lending professionnel | BIC |
| Airdrop reçu passivement | Non imposable à la réception, PV au moment de la cession |
| Rewards actifs (tâches à accomplir) | BNC ou salaire |

**Zone grise** : la doctrine DGFIP évolue. Vérifier les dernières positions BOFiP.

## Documentation à conserver

Pour 6 ans minimum (délai de reprise) :
- Historique complet des transactions (exports exchanges)
- Preuves des dates et prix d'acquisition
- Détail des échanges crypto-to-crypto (même non imposables)
- Transferts entre wallets (pour prouver la continuité du portefeuille)

## Références CGI / BOFiP

- Régime particulier crypto : art. 150 VH bis CGI
- Activité habituelle (BIC) : art. 34 CGI
- Méthode PAMC : art. 150 VH bis-II CGI
- Sursis échange crypto-crypto : art. 150 VH bis-I-2 CGI
- BOFiP : BOI-RPPM-PVBMC-30
