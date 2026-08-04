# Forge SDK - Keep JNI methods
-keepclassmembers class com.forge.sdk.** { *; }
-keep class com.forge.sdk.** { *; }

# Kotlin Serialization
-keepattributes *Annotation*, InnerClasses
-dontnote kotlinx.serialization.AnnotationsKt
-keepclassmembers class kotlinx.serialization.json.** {
    *** Companion;
}
-keepclasseswithmembers class kotlinx.serialization.json.** {
    kotlinx.serialization.KSerializer serializer(...);
}
-keep,includedescriptorclasses class com.amma.recognize.**$$serializer { *; }
-keepclassmembers class com.amma.recognize.** {
    *** Companion;
}
-keepclasseswithmembers class com.amma.recognize.** {
    kotlinx.serialization.KSerializer serializer(...);
}
