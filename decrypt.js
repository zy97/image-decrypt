//依赖crpto-js.js
const media_key = CryptoJS[String.fromCharCode(101) + String.fromCharCode(110) + String.fromCharCode(99)][String.fromCharCode(85) + String.fromCharCode(116) + String.fromCharCode(102) + String.fromCharCode(56)][`${String.fromCharCode(112)}arse`]("102_53_100_57_54_53_100_102_55_53_51_51_54_50_55_48"
    .split("_")
    .map((a) => String.fromCharCode(parseInt(a)))
    .join(""));
console.log(media_key);
// const media_key = '66356439363564663735333336323730';
const media_iv = CryptoJS[String.fromCharCode(101) + String.fromCharCode(110) + String.fromCharCode(99)][String.fromCharCode(85) + String.fromCharCode(116) + String.fromCharCode(102) + String.fromCharCode(56)][`${String.fromCharCode(112)}arse`]("57_55_98_54_48_51_57_52_97_98_99_50_102_98_101_49"
    .split("_")
    .map((a) => String.fromCharCode(parseInt(a)))
    .join(""));
console.log(media_iv);

// const media_iv = '39376236303339346162633266626531';
function decryptImage(word) {
    const decrypt = CryptoJS[String.fromCharCode(65) + String.fromCharCode(69) + String.fromCharCode(83)]["100_101_99_114_121_112_116"
        .split("_")
        .map((a) => String.fromCharCode(parseInt(a)))
        .join("")](word, media_key, {
            iv: media_iv,
            mode: CryptoJS["109_111_100_101"
                .split("_")
                .map((a) => String.fromCharCode(parseInt(a)))
                .join("")][String.fromCharCode(67) + String.fromCharCode(66) + String.fromCharCode(67)],
            padding: CryptoJS[`${String.fromCharCode(112)}ad`][`${String.fromCharCode(78)}o${String.fromCharCode(80)}adding`],
        });
    return decrypt.toString(CryptoJS.enc.Base64);
}