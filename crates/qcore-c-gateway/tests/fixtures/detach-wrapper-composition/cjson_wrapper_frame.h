/*
  Copyright (c) 2009-2017 Dave Gamble and cJSON contributors

  Permission is hereby granted, free of charge, to any person obtaining a copy
  of this software and associated documentation files (the "Software"), to deal
  in the Software without restriction, including without limitation the rights
  to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
  copies of the Software, and to permit persons to whom the Software is
  furnished to do so, subject to the following conditions:

  The above copyright notice and this permission notice shall be included in
  all copies or substantial portions of the Software.

  THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
  IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
  FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
  AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
  LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
  OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
  THE SOFTWARE.
*/


/* Retained preprocessed cJSON record, lookups, unlink frame and the six
   detach/delete wrappers at 66910c10 (lib/librdkafka-2.15.0/src/cJSON.c
   :1907-1976 and :2243-2316). cJSON_Delete is declared only here. */
typedef long unsigned int size_t;
typedef int cJSON_bool;
typedef struct cJSON
{
    struct cJSON *next;
    struct cJSON *prev;
    struct cJSON *child;
    int type;
    char *valuestring;
    int valueint;
    double valuedouble;
    char *string;
} cJSON;
extern int strcmp(const char *__s1, const char *__s2);
static int case_insensitive_strcmp(const unsigned char *string1, const unsigned char *string2);
void cJSON_Delete(cJSON *item);
static cJSON* get_array_item(const cJSON *array, size_t index)
{
    cJSON *current_child = ((void*)0);
    if (array == ((void*)0))
    {
        return ((void*)0);
    }
    current_child = array->child;
    while ((current_child != ((void*)0)) && (index > 0))
    {
        index--;
        current_child = current_child->next;
    }
    return current_child;
}
cJSON * cJSON_GetArrayItem(const cJSON *array, int index)
{
    if (index < 0)
    {
        return ((void*)0);
    }
    return get_array_item(array, (size_t)index);
}
static cJSON *get_object_item(const cJSON * const object, const char * const name, const cJSON_bool case_sensitive)
{
    cJSON *current_element = ((void*)0);
    if ((object == ((void*)0)) || (name == ((void*)0)))
    {
        return ((void*)0);
    }
    current_element = object->child;
    if (case_sensitive)
    {
        while ((current_element != ((void*)0)) && (current_element->string != ((void*)0)) && (strcmp(name, current_element->string) != 0))
        {
            current_element = current_element->next;
        }
    }
    else
    {
        while ((current_element != ((void*)0)) && (case_insensitive_strcmp((const unsigned char*)name, (const unsigned char*)(current_element->string)) != 0))
        {
            current_element = current_element->next;
        }
    }
    if ((current_element == ((void*)0)) || (current_element->string == ((void*)0))) {
        return ((void*)0);
    }
    return current_element;
}
cJSON * cJSON_GetObjectItem(const cJSON * const object, const char * const string)
{
    return get_object_item(object, string, 0);
}
cJSON * cJSON_GetObjectItemCaseSensitive(const cJSON * const object, const char * const string)
{
    return get_object_item(object, string, 1);
}
cJSON * cJSON_DetachItemViaPointer(cJSON *parent, cJSON * const item)
{
    if ((parent == ((void*)0)) || (item == ((void*)0)) || (item != parent->child && item->prev == ((void*)0)))
    {
        return ((void*)0);
    }
    if (item != parent->child)
    {
        item->prev->next = item->next;
    }
    if (item->next != ((void*)0))
    {
        item->next->prev = item->prev;
    }
    if (item == parent->child)
    {
        parent->child = item->next;
    }
    else if (item->next == ((void*)0))
    {
        parent->child->prev = item->prev;
    }
    item->prev = ((void*)0);
    item->next = ((void*)0);
    return item;
}
cJSON * cJSON_DetachItemFromArray(cJSON *array, int which)
{
    if (which < 0)
    {
        return ((void*)0);
    }
    return cJSON_DetachItemViaPointer(array, get_array_item(array, (size_t)which));
}
void cJSON_DeleteItemFromArray(cJSON *array, int which)
{
    cJSON_Delete(cJSON_DetachItemFromArray(array, which));
}
cJSON * cJSON_DetachItemFromObject(cJSON *object, const char *string)
{
    cJSON *to_detach = cJSON_GetObjectItem(object, string);
    return cJSON_DetachItemViaPointer(object, to_detach);
}
cJSON * cJSON_DetachItemFromObjectCaseSensitive(cJSON *object, const char *string)
{
    cJSON *to_detach = cJSON_GetObjectItemCaseSensitive(object, string);
    return cJSON_DetachItemViaPointer(object, to_detach);
}
void cJSON_DeleteItemFromObject(cJSON *object, const char *string)
{
    cJSON_Delete(cJSON_DetachItemFromObject(object, string));
}
void cJSON_DeleteItemFromObjectCaseSensitive(cJSON *object, const char *string)
{
    cJSON_Delete(cJSON_DetachItemFromObjectCaseSensitive(object, string));
}
