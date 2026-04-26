# Revenus du capital (RCM, dividendes, plus-values mobilières)

## PFU vs barème : l'arbitrage fondamental

Les revenus mobiliers (dividendes, intérêts, plus-values de titres) sont soumis au choix :

| Régime | Taux | Caractéristiques |
|--------|------|------------------|
| **PFU (flat tax)** | 30% (12,8% IR + 17,2% PS) | Par défaut. Pas d'abattement. Pas de CSG déductible. |
| **Barème progressif** | Selon TMI | Sur option **globale** (tous les revenus du capital de l'année). Abattement 40% sur dividendes. CSG déductible 6,8% en N+1. |

Voir `data/pfu-prelevements-sociaux.json` pour les taux exacts.

### Règles d'orientation rapide

| TMI | Recommandation | Raison |
|-----|---------------|--------|
| 0% ou 11% | Barème | Tranche basse + abattement 40% dividendes + CSG déductible |
| 30% | À chiffrer | Selon composition : dividendes (abattement 40%) vs intérêts/PV (pas d'abattement) |
| 41% ou 45% | PFU | Flat tax 12,8% < TMI 41%/45% |

**Règle absolue** : l'option barème est **globale et irrévocable pour l'année**. Elle engage TOUS les revenus du capital du foyer. Ne jamais recommander sans avoir vérifié la composition complète.

### Exemple de calcul comparatif

Foyer célibataire, TMI 30%, dividendes 10 000 € :

**Sous PFU** :
- IR : 10 000 × 12,8% = 1 280 €
- PS : 10 000 × 17,2% = 1 720 €
- Total : 3 000 €

**Sous barème** :
- Assiette IR : 10 000 × (1 − 0,40) = 6 000 €
- IR : 6 000 × 30% = 1 800 €
- PS : 10 000 × 17,2% = 1 720 €
- CSG déductible N+1 : 10 000 × 6,8% × 30% (économie) = 204 € (N+1)
- Total net : 3 520 − 204 = **3 316 €**

→ Ici, PFU avantageux (3 000 € < 3 316 €) malgré l'abattement 40%.

Mais si le même foyer a aussi **5 000 € de PV mobilière** sans abattement :
- PFU : 5 000 × 30% = 1 500 €
- Barème : 5 000 × 30% IR + 5 000 × 17,2% PS = 2 360 €
- → PFU toujours plus favorable

## Types de revenus du capital

### Dividendes (case 2DC)

- Par défaut : PFU 30%
- Option barème : abattement 40% puis IR barème + PS 17,2%
- **Dividendes étrangers** : peuvent avoir été soumis à une retenue à la source dans le pays d'origine — crédit d'impôt possible sous convention fiscale

### Intérêts, RCM (case 2TR)

- Obligations, crowdfunding immobilier (intérêts), livrets fiscalisés, comptes à terme
- Soumis au PFU par défaut ou barème sur option
- **Pas d'abattement** — contrairement aux dividendes
- **Crowdfunding immobilier** : imposé comme RCM, pas comme revenus fonciers, même si sous-jacent immobilier
- **Livrets réglementés** (Livret A, LDDS, LEP) : exonérés d'IR et de PS → à ne pas confondre

### Plus-values mobilières (case 3VG)

- Gain net de cession de titres (actions, parts sociales, OPC)
- PFU par défaut ou barème sur option
- Voir `data/plus-values-mobilieres-crypto.json` pour détails
- **Abattements durée de détention** : uniquement pour titres acquis **avant 2018** ET option barème
- **Abattement dirigeant retraite** : 500 000 € forfaitaires sous conditions strictes (cession de titres, départ en retraite)

### Crypto-actifs

Régime distinct : méthode PAMC (prix acquisition moyen pondéré) sur l'ensemble du portefeuille, formulaire 2086, seuil d'exonération cessions < 305 €/an. Détail dans la section crypto listée depuis SKILL.md.

## Prélèvements sociaux : couche distincte de l'IR

**À ne jamais confondre avec l'IR.** Les PS sont prélevés en plus sur la quasi-totalité des revenus du capital.

- Taux global : 17,2% (ou 18,6% selon LFSS 2026 — vérifier)
- Composition : CSG + CRDS + prélèvement de solidarité
- **Sous PFU** : PS inclus dans le taux global de 30%
- **Sous barème** : PS séparés de l'IR (mais CSG 6,8% déductible en N+1)

**Exceptions de taux (17,2% au lieu du taux courant)** :
- AV et contrats de capitalisation anciens
- CEL/PEL ouverts avant le 31/12/2017
- PEP

### CSG déductible

- Montant : 6,8% de la base
- **Condition** : uniquement sous option barème progressif
- **Zéro déductible sous PFU**
- Imputée en N+1 sur le RNI → économie = CSG déductible × TMI N+1

## Enveloppes fiscales spécifiques

**PEA** : exonération IR après 5 ans, PS 17,2% restent dus.
**AV** : abattement annuel après 8 ans, fiscalité selon date des versements.

Règles complètes (retrait avant/après 5 ans PEA, abattements AV par tranche de versement, 152 500 € succession AV) dans la section PEA/AV listée depuis SKILL.md.

## Pièges fréquents

1. **Confondre IR et PS** dans une simulation — conduit à sous-estimer la charge d'environ 17 points.
2. **Oublier l'option barème globale** — choisir le barème pour les dividendes implique le barème aussi pour les intérêts et PV mobilières.
3. **CSG déductible sous PFU** — elle n'existe pas.
4. **Abattement 40% sous PFU** — n'existe que sous barème.
5. **Retenue à la source étrangère** — créditer contre l'IR français (conventions fiscales) — à ne pas oublier.

## Références CGI / BOFiP

- PFU : art. 200 A CGI
- Option barème : art. 200 A-2 CGI
- Abattement dividendes 40% : art. 158-3° CGI
- Plus-values mobilières : art. 150-0 A à 150-0 D CGI
- Prélèvements sociaux : art. L. 136-1 et s. CSS
- BOFiP : BOI-RPPM-RCM (dividendes/RCM) et BOI-RPPM-PVBMI (PV mobilières)
