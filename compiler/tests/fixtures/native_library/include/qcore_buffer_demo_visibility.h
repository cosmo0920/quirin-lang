/* The export-decoration header of the fixture native library
 * `qcore_buffer_demo` (card `NATIVE-LIBRARY-HEADER-DECORATION`).
 *
 * This is the shape Libevent's `event2/visibility.h` has, reproduced here so
 * that the derivation is pinned in this repository and not only against a
 * submodule. `qcore_buffer_demo_decorated.h` writes
 * `QCORE_BUFFER_DEMO_EXPORT` before every declaration, and nothing in that
 * header says what the name means.
 *
 * What makes the name erasable is read here, and only here:
 *
 *   - every `#define` of it is object-like;
 *   - every body is a decoration that changes neither the declared type, the
 *     parameter vector nor the linkage name — `__attribute__((visibility(…)))`,
 *     `__declspec(dllexport)`, `__global`, and the empty body;
 *   - no arm of the conditional structure leaves it undefined, because every
 *     group below has an `#else`.
 *
 * The reader evaluates none of these conditions. It does not need to: whichever
 * arm a consumer's configuration selects, the name is defined and its expansion
 * is erasable, so deleting the name is the expansion.
 */
#ifndef QCORE_BUFFER_DEMO_VISIBILITY_H
#define QCORE_BUFFER_DEMO_VISIBILITY_H

#if defined(QCORE_BUFFER_DEMO_SHARED)

# if defined(__SUNPRO_C)
#  define QCORE_BUFFER_DEMO_EXPORT __global
# elif defined(__GNUC__)
#  define QCORE_BUFFER_DEMO_EXPORT __attribute__((visibility("default")))
# elif defined(_MSC_VER)
#  define QCORE_BUFFER_DEMO_EXPORT __declspec(dllexport)
# else
#  define QCORE_BUFFER_DEMO_EXPORT /* unknown compiler */
# endif

#else /* QCORE_BUFFER_DEMO_SHARED */

# define QCORE_BUFFER_DEMO_EXPORT

#endif /* QCORE_BUFFER_DEMO_SHARED */

#endif /* QCORE_BUFFER_DEMO_VISIBILITY_H */
